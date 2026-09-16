use std::cell::RefCell;
use std::rc::Rc;

use calloop::EventLoop;
use calloop::channel;
use calloop::channel::Event;
use calloop::channel::Sender;
use calloop_wayland_source::WaylandSource;
use slint::platform::{EventLoopProxy, Platform as SlintPlatform, WindowAdapter};
use slint::{EventLoopError, PhysicalSize, PlatformError};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::globals::registry_queue_init;
use wayland_client::{Connection, QueueHandle};

use crate::LayerShellOptions;
use crate::render::{RenderBackend, SoftwareRenderBackend};
use crate::wayland::AppState;
use crate::window_adapter::LayerWindowAdapter;

enum ProxyMessage {
    Invoke(Box<dyn FnOnce() + Send>),
    Quit,
}

/// Slint `Platform` implementation for Wayland.
pub struct WaylandPlatform {
    event_loop: RefCell<Option<EventLoop<'static, AppState>>>,
    app_state: RefCell<Option<AppState>>,
    layer: LayerSurface,
    qh: QueueHandle<AppState>,
    render_backend: RefCell<Option<Box<dyn RenderBackend>>>,
    window_adapter: RefCell<Option<Rc<LayerWindowAdapter>>>,
    initial_size: PhysicalSize,
    fallback_size: (u32, u32),
    proxy_sender: Sender<ProxyMessage>,
}

impl WaylandPlatform {
    pub(crate) fn new(options: LayerShellOptions) -> Result<Self, PlatformError> {
        let connection = Connection::connect_to_env().map_err(|e| {
            PlatformError::Other(format!("failed to connect to Wayland display: {e}"))
        })?;
        let (globals, mut event_queue) = registry_queue_init::<AppState>(&connection)
            .map_err(|e| PlatformError::Other(format!("failed to read Wayland registry: {e}")))?;
        let qh = event_queue.handle();

        let compositor = CompositorState::bind(&globals, &qh)
            .map_err(|e| PlatformError::Other(format!("wl_compositor is not available: {e}")))?;
        let layer_shell = LayerShell::bind(&globals, &qh).map_err(|e| {
            PlatformError::Other(format!(
                "wlr-layer-shell is not available on this compositor: {e}"
            ))
        })?;
        let shm = Shm::bind(&globals, &qh)
            .map_err(|e| PlatformError::Other(format!("wl_shm is not available: {e}")))?;
        let seats = SeatState::new(&globals, &qh);
        let outputs = OutputState::new(&globals, &qh);
        let registry = RegistryState::new(&globals);

        let surface = compositor.create_surface(&qh);
        let layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            options.layer,
            Some(options.namespace),
            None,
        );
        layer.set_anchor(options.anchor);
        layer.set_keyboard_interactivity(options.keyboard_interactivity);
        let (top, right, bottom, left) = options.margin;
        layer.set_margin(top, right, bottom, left);
        if let Some(zone) = options.exclusive_zone {
            layer.set_exclusive_zone(zone);
        }
        let fallback_size = (options.size.0.unwrap_or(256), options.size.1.unwrap_or(32));
        layer.set_size(options.size.0.unwrap_or(0), options.size.1.unwrap_or(0));
        layer.commit();

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

        let initial_size = PhysicalSize::new(app_state.width, app_state.height);
        let render_backend =
            SoftwareRenderBackend::new(layer.clone(), qh.clone(), &app_state.shm, initial_size)?;

        Ok(Self {
            event_loop: RefCell::new(Some(event_loop)),
            app_state: RefCell::new(Some(app_state)),
            layer,
            qh,
            render_backend: RefCell::new(Some(Box::new(render_backend))),
            window_adapter: RefCell::new(None),
            initial_size,
            fallback_size,
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
        let adapter = LayerWindowAdapter::new(
            self.layer.clone(),
            self.qh.clone(),
            render_backend,
            self.initial_size,
            self.fallback_size,
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
