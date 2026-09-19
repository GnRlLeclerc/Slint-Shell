use std::cell::RefCell;
use std::rc::Rc;

use calloop::EventLoop;
use calloop::channel;
use calloop::channel::Event;
use calloop::channel::Sender;
use calloop_wayland_source::WaylandSource;
use slint::platform::{EventLoopProxy, Platform as SlintPlatform, WindowAdapter};
use slint::{EventLoopError, LogicalSize, PhysicalSize, PlatformError};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::Shm;
use wayland_client::globals::registry_queue_init;
use wayland_client::{Connection, QueueHandle};

use crate::Options;
use crate::render::{RenderBackend, SoftwareRenderBackend};
use crate::surface::Surface;
use crate::wayland::AppState;
use crate::window_adapter::ShellWindowAdapter;

enum ProxyMessage {
    Invoke(Box<dyn FnOnce() + Send>),
    Quit,
}

/// Slint Platform for Wayland.
pub struct WaylandPlatform {
    event_loop: RefCell<Option<EventLoop<'static, AppState>>>,
    app_state: RefCell<Option<AppState>>,
    surface: Surface,
    qh: QueueHandle<AppState>,
    render_backend: RefCell<Option<Box<dyn RenderBackend>>>,
    window_adapter: RefCell<Option<Rc<ShellWindowAdapter>>>,
    initial_size: PhysicalSize,
    fallback_size: (u32, u32),
    scale: i32,
    proxy_sender: Sender<ProxyMessage>,
}

impl WaylandPlatform {
    pub(crate) fn new(options: Options) -> Result<Self, PlatformError> {
        let connection = Connection::connect_to_env().map_err(|e| {
            PlatformError::Other(format!("failed to connect to Wayland display: {e}"))
        })?;
        let (globals, mut event_queue) = registry_queue_init::<AppState>(&connection)
            .map_err(|e| PlatformError::Other(format!("failed to read Wayland registry: {e}")))?;
        let qh = event_queue.handle();

        let compositor = CompositorState::bind(&globals, &qh)
            .map_err(|e| PlatformError::Other(format!("wl_compositor is not available: {e}")))?;
        let shm = Shm::bind(&globals, &qh)
            .map_err(|e| PlatformError::Other(format!("wl_shm is not available: {e}")))?;
        let seats = SeatState::new(&globals, &qh);
        let outputs = OutputState::new(&globals, &qh);
        let registry = RegistryState::new(&globals);

        let (surface, fallback_size) = Surface::new(&options, &globals, &qh, &compositor)?;
        // Initial commit with no buffer to get the first configure event from the compositor
        surface.commit();

        let event_loop: EventLoop<'static, AppState> = EventLoop::try_new()
            .map_err(|e| PlatformError::Other(format!("failed to create the event loop: {e}")))?;
        let loop_handle = event_loop.handle();

        let (proxy_sender, proxy_channel) = channel::channel::<ProxyMessage>();
        loop_handle
            .insert_source(
                proxy_channel,
                |event, _, app_state: &mut AppState| match event {
                    Event::Msg(ProxyMessage::Invoke(f)) => f(),
                    Event::Msg(ProxyMessage::Quit) => app_state.exit = true,
                    Event::Closed => {}
                },
            )
            .map_err(|e| {
                PlatformError::Other(format!("failed to set up the event loop proxy: {e}"))
            })?;

        let mut app_state = AppState {
            registry,
            seats,
            outputs,
            shm,
            window_adapter: std::rc::Weak::new(),
            width: fallback_size.0,
            height: fallback_size.1,
            scale: 1,
            exit: false,
            keyboard: None,
            loop_handle: loop_handle.clone(),
            pointer: None,
        };

        event_queue
            .roundtrip(&mut app_state)
            .map_err(|e| PlatformError::Other(format!("initial Wayland roundtrip failed: {e}")))?;

        WaylandSource::new(connection, event_queue)
            .insert(loop_handle)
            .map_err(|e| {
                PlatformError::Other(format!(
                    "failed to register the Wayland connection with the event loop: {e}"
                ))
            })?;

        let scale = app_state.scale;
        let initial_size = LogicalSize::new(app_state.width as f32, app_state.height as f32)
            .to_physical(scale as f32);
        let render_backend =
            SoftwareRenderBackend::new(surface.clone(), qh.clone(), &app_state.shm, initial_size)?;

        Ok(Self {
            event_loop: RefCell::new(Some(event_loop)),
            app_state: RefCell::new(Some(app_state)),
            surface,
            qh,
            render_backend: RefCell::new(Some(Box::new(render_backend))),
            window_adapter: RefCell::new(None),
            initial_size,
            fallback_size,
            scale,
            proxy_sender,
        })
    }
}

impl SlintPlatform for WaylandPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        if let Some(existing) = self.window_adapter.borrow().as_ref() {
            return Ok(existing.clone());
        }
        let render_backend = self
            .render_backend
            .borrow_mut()
            .take()
            .expect("render backend already taken by a previous create_window_adapter() call");
        let adapter = ShellWindowAdapter::new(
            self.surface.clone(),
            self.qh.clone(),
            render_backend,
            self.initial_size,
            self.fallback_size,
            self.scale,
        );
        self.app_state
            .borrow_mut()
            .as_mut()
            .expect("app_state consumed")
            .window_adapter = Rc::downgrade(&adapter);
        *self.window_adapter.borrow_mut() = Some(adapter.clone());
        Ok(adapter)
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        let mut event_loop =
            self.event_loop.borrow_mut().take().ok_or_else(|| {
                PlatformError::Other("run_event_loop() called more than once".into())
            })?;
        let mut app_state = self
            .app_state
            .borrow_mut()
            .take()
            .expect("app_state consumed by create_window_adapter");

        while !app_state.exit {
            let timeout = slint::platform::duration_until_next_timer_update();
            event_loop
                .dispatch(timeout, &mut app_state)
                .map_err(|e| PlatformError::Other(format!("event loop dispatch failed: {e}")))?;
            slint::platform::update_timers_and_animations();
        }
        Ok(())
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(Proxy {
            sender: self.proxy_sender.clone(),
        }))
    }
}

struct Proxy {
    sender: Sender<ProxyMessage>,
}

impl EventLoopProxy for Proxy {
    fn quit_event_loop(&self) -> Result<(), EventLoopError> {
        self.sender
            .send(ProxyMessage::Quit)
            .map_err(|_| EventLoopError::EventLoopTerminated)
    }

    fn invoke_from_event_loop(
        &self,
        event: Box<dyn FnOnce() + Send>,
    ) -> Result<(), EventLoopError> {
        self.sender
            .send(ProxyMessage::Invoke(event))
            .map_err(|_| EventLoopError::EventLoopTerminated)
    }
}
