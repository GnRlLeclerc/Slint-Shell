//! Window example

use slint_shell::{Options, WindowOptions};

slint::slint! {
    export component MainWindow inherits Window {
        in-out property <int> counter: 0;

        TouchArea {
            clicked => { counter += 1; }

            Rectangle {
                background: parent.pressed ? #45475a : #313244;

                Text {
                    text: "clicks: " + counter;
                    color: white;
                    vertical-alignment: center;
                    horizontal-alignment: center;
                }
            }
        }
    }
}

fn main() {
    slint_shell::init(Options::Window(WindowOptions {
        title: "slint-shell demo window",
        app_id: "slint-shell-example-window",
        size: (Some(400), Some(300)),
        ..Default::default()
    }))
    .expect("failed to initialize the Wayland xdg_toplevel backend");

    let window = MainWindow::new().expect("failed to create the UI");
    window.run().expect("event loop failed");
}
