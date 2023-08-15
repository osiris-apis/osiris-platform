//! UI Counter via Makepad
//!
//! XXX

use makepad_widgets::*;

live_design! {
    import makepad_widgets::desktop_window::DesktopWindow;
    import makepad_widgets::label::Label;

    App = {{App}} {
        ui: <DesktopWindow> {
            show_bg: true,
            layout: {
                flow: Down,
                spacing: 20,
                align: {
                    x: 0.5,
                    y: 0.5
                }
            },
            walk: {
                width: Fill,
                height: Fill
            },
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return mix(#7, #3, self.geom_pos.y);
                }
            },
            label = <Label> {
                draw_label: {
                    color: #f
                },
                label: "Value: 0"
            }
        }
    }
}

#[derive(Live)]
pub struct App {
    #[live] ui: WidgetRef,
}

impl LiveHook for App {
    fn before_live_design(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, ev: &Event) {
        if let Event::Draw(ev) = ev {
            self.ui.draw_widget_all(&mut Cx2d::new(cx, ev))
        } else {
            // Nothing to do.
        }
    }
}

app_main!(App);
