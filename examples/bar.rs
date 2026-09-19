//! Top bar example

use slint_shell::{Anchor, Layer, LayerShellOptions, Options};

slint::slint! {
    export component Bar inherits Window {
        in-out property <int> counter: 0;
        background: #1e1e2eee;

        TouchArea {
            clicked => { counter += 1; }

            Rectangle {
                background: parent.pressed ? #45475a : transparent;

                Text {
                    text: "slint-shell demo bar — clicks: " + counter;
                    color: white;
                    vertical-alignment: center;
                    horizontal-alignment: center;
                }
            }
        }
    }
}

fn main() {
    slint_shell::init(Options::Layer(LayerShellOptions {
        namespace: "slint-shell-example-bar",
        layer: Layer::Top,
        anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        size: (None, Some(36)),
        exclusive_zone: Some(36),
        ..Default::default()
    }))
    .expect("failed to initialize the Wayland layer-shell backend");

    let bar = Bar::new().expect("failed to create the UI");
    bar.run().expect("event loop failed");
}
