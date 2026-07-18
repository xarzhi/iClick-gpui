use gpui::Bounds;
use gpui::WindowOptions;
use gpui::*;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::*;
pub struct IClick;

impl Render for IClick {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            TabBar::new("tabs")
                .small()
                .selected_index(0)
                .segmented()
                .on_click(|selected_index, _, _| {
                    println!("Tab {} selected", selected_index);
                })
                .child(Tab::new().label("操作一"))
                .child(Tab::new().label("操作二"))
                .child(Tab::new().label("Settings")),
        )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx: &mut AsyncApp| {
            cx.update(|cx: &mut App| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds {
                            origin: Point {
                                x: gpui::px(580.0),
                                y: gpui::px(580.0),
                            },
                            size: gpui::Size {
                                width: gpui::px(580.0),
                                height: gpui::px(580.0),
                            },
                        })),
                        titlebar: Some(TitlebarOptions {
                            title: Some("iClick-gpui".into()),
                            ..Default::default()
                        }),
                        is_resizable: false,
                        ..WindowOptions::default()
                    },
                    |window, cx| {
                        let view = cx.new(|_| IClick);
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
