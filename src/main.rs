#![windows_subsystem = "windows"]

use gpui::Bounds;
use gpui::WindowOptions;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::*;
use std::vec;

pub struct IClick {
    active_tab: usize,
    tabs: Vec<String>,
    loop_count_input: Entity<InputState>,
    fuzzy_click_input: Entity<InputState>,
}

impl Render for IClick {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                TabBar::new("tabs")
                    .small()
                    .selected_index(self.active_tab)
                    .segmented()
                    .on_click(cx.listener(|view, index, _, cx| {
                        view.active_tab = *index;
                        cx.notify();
                    }))
                    .children(self.tabs.iter().map(|tab_name| {
                        Tab::new()
                            .label(tab_name.clone())
                            .cursor(CursorStyle::PointingHand)
                    })),
            )
            .child(if self.active_tab == 0 {
                div()
                    .p(px(5.0))
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .mb(px(10.0))
                            .child(
                                div()
                                    .border(px(1.0))
                                    .border_color(cx.theme().border)
                                    .w(px(200.0))
                                    .h(px(250.0))
                                    .mr(px(10.0))
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .h(px(130.0))
                                            .mb(px(10.0))
                                            .w_full()
                                            .border(px(1.0))
                                            .border_color(cx.theme().border),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(120.0))
                                            .border(px(1.0))
                                            .border_color(cx.theme().border),
                                    ),
                            )
                            .child(
                                div()
                                    .border(px(1.0))
                                    .border_color(cx.theme().border)
                                    .flex_1()
                                    .h(px(250.0))
                                    .flex()
                                    .justify_center()
                                    .p(px(5.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .flex()
                                            .flex_col()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("窗口置顶"),
                                                    )
                                                    .child(
                                                        Checkbox::new("terms")
                                                            .label("")
                                                            .checked(false),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("前台操作"),
                                                    )
                                                    .child(
                                                        Checkbox::new("terms")
                                                            .label("")
                                                            .checked(false),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("循环次数"),
                                                    )
                                                    .child(
                                                        div().w(px(80.0)).child(Input::new(
                                                            &self.loop_count_input,
                                                        )),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("模糊点击"),
                                                    )
                                                    .child(
                                                        Checkbox::new("terms")
                                                            .label("")
                                                            .checked(false),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("操作间隔"),
                                                    )
                                                    .child(
                                                        div().w(px(80.0)).child(
                                                            Input::new(&self.fuzzy_click_input)
                                                                .suffix(div().child("px")),
                                                        ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("开始/暂停快捷键"),
                                                    )
                                                    .child(
                                                        Checkbox::new("terms")
                                                            .label("")
                                                            .checked(false),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .text_size(px(14.0))
                                            .flex()
                                            .flex_col()
                                            .justify_between()
                                            .items_end()
                                            .child(
                                                Button::new("start_pause")
                                                    .label("开始")
                                                    .w(px(140.0))
                                                    .cursor(CursorStyle::PointingHand),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .justify_between()
                                                    .w(px(140.0))
                                                    .child(
                                                        Button::new("start_pause")
                                                            .label("读取")
                                                            .small()
                                                            .mr(px(10.0))
                                                            .cursor(CursorStyle::PointingHand),
                                                    )
                                                    .child(
                                                        Button::new("start_pause")
                                                            .label("保存")
                                                            .small()
                                                            .cursor(CursorStyle::PointingHand),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("开始录制"),
                                                    )
                                                    .child(
                                                        div().w(px(80.0)).child(
                                                            Input::new(&self.fuzzy_click_input)
                                                                .suffix(div().child("px")),
                                                        ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("模糊范围"),
                                                    )
                                                    .child(
                                                        div().w(px(80.0)).child(
                                                            Input::new(&self.fuzzy_click_input)
                                                                .suffix(div().child("px")),
                                                        ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .mr(px(10.0))
                                                            .flex()
                                                            .items_center()
                                                            .child("循环间隔"),
                                                    )
                                                    .child(
                                                        div().w(px(80.0)).child(
                                                            Input::new(&self.fuzzy_click_input)
                                                                .suffix(div().child("px")),
                                                        ),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(150.0))
                            .border(px(1.0))
                            .border_color(cx.theme().border),
                    )
            } else if self.active_tab == 1 {
                div().size_full().bg(rgb(0xf6f6f6))
            } else {
                div()
            })
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
                        let loop_count_input = cx.new(|cx| InputState::new(window, cx));
                        let fuzzy_click_input = cx.new(|cx| InputState::new(window, cx));

                        let view = cx.new(|_| IClick {
                            active_tab: 0,
                            tabs: vec![
                                "操作一".to_string(),
                                "操作二".to_string(),
                                "设置".to_string(),
                            ],
                            loop_count_input,
                            fuzzy_click_input,
                        });
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
