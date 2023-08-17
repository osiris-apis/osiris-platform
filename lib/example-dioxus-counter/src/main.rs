//! UI Counter via Dioxus
//!
//! A simple UI counter implemented via Dioxus.

use dioxus;
use dioxus_desktop;

use dioxus::html as dioxus_elements;

fn app(cx: dioxus::core::Scope) -> dioxus::core::Element {
    cx.render(dioxus::core_macro::rsx! {
        div {
            "Hello, world!"
        }
    })
}

fn main() {
    dioxus_desktop::launch(app);
}
