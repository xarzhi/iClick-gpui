// #![windows_subsystem = "windows"]
use gpui::Bounds;
use gpui::SharedString;
use gpui::WindowOptions;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputState};
use gpui_component::radio::{RadioGroup};
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_component::*;
use gpui_component::{Icon, IconName};
use gpui_component::tooltip::Tooltip;
use gpui_component::kbd::Kbd;
use std::sync::LazyLock;
use std::vec;

pub struct IClick {
    active_tab: usize,
    tabs: Vec<String>,
    always_on_top: bool,
    front_opt: bool,
    opt_cate: Option<usize>,
    fuzzy_click: bool,
    hide_when_choose: bool,
    watch_hotkey: bool,
    loop_count_input: Entity<InputState>,  // 循环次数
    fuzzy_round_input: Entity<InputState>,	// 模糊范围
    loop_gap_ipt: Entity<InputState>,	// 循环间隔
    opt_gap_ipt: Entity<InputState>,	// 循环间隔
    data_table: Entity<TableState<MyTableDelegate>>,
}

#[derive(Clone, Debug)]
pub enum OptCate {
    鼠标,
    键盘,
}
#[derive(Clone, Debug)]
pub enum MouseOptType {
    左键单击,
    左键双击,
    滚轮上滑,
    滚轮下滑,
    滚轮单击,
    右键单击,
    右键双击,
}
impl SelectItem for MouseOptType {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        match self {
            MouseOptType::左键单击 => "左键单击".into(),
            MouseOptType::左键双击 => "左键双击".into(),
            MouseOptType::滚轮上滑 => "滚轮上滑".into(),
            MouseOptType::滚轮下滑 => "滚轮下滑".into(),
            MouseOptType::滚轮单击 => "滚轮单击".into(),
            MouseOptType::右键单击 => "右键单击".into(),
            MouseOptType::右键双击 => "右键双击".into(),
        }
    }
    fn value(&self) -> &Self::Value {
        // value() 需要返回引用，用一个 static 缓存
        match self {
            MouseOptType::左键单击 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "鼠标".into());
                &V
            }
            MouseOptType::左键双击 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "左键双击".into());
                &V
            }
            MouseOptType::滚轮上滑 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "滚轮上滑".into());
                &V
            }
            MouseOptType::滚轮下滑 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "滚轮下滑".into());
                &V
            }
            MouseOptType::滚轮单击 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "滚轮单击".into());
                &V
            }
            MouseOptType::右键单击 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "右键单击".into());
                &V
            }
            MouseOptType::右键双击 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "右键双击".into());
                &V
            }
       
        }
    }
}
#[derive(Clone, Debug)]

struct MyData {
    x: usize,
    y: usize,
    opt_cate: OptCate,
    opt_type: String,
    win_title: Option<String>,
    delay: usize,
    execute_time: usize,
}

impl SelectItem for OptCate {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        match self {
            OptCate::鼠标 => "鼠标".into(),
            OptCate::键盘 => "键盘".into(),
        }
    }
    fn value(&self) -> &Self::Value {
        // value() 需要返回引用，用一个 static 缓存
        match self {
            OptCate::鼠标 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "鼠标".into());
                &V
            }
            OptCate::键盘 => {
                static V: LazyLock<SharedString> = LazyLock::new(|| "键盘".into());
                &V
            }
        }
    }
}

struct MyTableDelegate {
    data: Vec<MyData>,
    columns: Vec<Column>,
    execute_time_inputs: Vec<Entity<InputState>>,
    opt_cate_selects: Vec<Entity<SelectState<SearchableVec<OptCate>>>>,
    opt_type_selects: Vec<Entity<SelectState<SearchableVec<MouseOptType>>>>,
    opt_cate_subscriptions: Vec<Subscription>,
}

impl MyTableDelegate {
    fn new(_window: &mut Window, _cx: &mut App) -> Self {
        let  data: Vec<MyData> = vec![];
        let  execute_time_inputs: Vec<Entity<InputState>> = vec![];
        let  opt_cate_selects: Vec<Entity<SelectState<SearchableVec<OptCate>>>> = vec![];
        let  opt_type_selects: Vec<Entity<SelectState<SearchableVec<MouseOptType>>>> = vec![];
        let  opt_cate_subscriptions: Vec<Subscription> = vec![];
        // let options = SearchableVec::new(vec![OptCate::鼠标, OptCate::键盘]);
        // for i in 0..3 {
        //     data.push(MyData {
        //         x: i + 1,
        //         y: i + 1,
        //         opt_cate: OptCate::鼠标,
        //         opt_type: "左键单击".to_string(),
        //         win_title: Some("chrome".to_string()),
        //         delay: 1,
        //         execute_time: 1,
        //     });

        //     execute_time_inputs.push(cx.new(|cx| {
        //         let mut state = InputState::new(window, cx);
        //         state.set_value("", window, cx);
        //         state
        //     }));

        //     opt_cate_selects.push(cx.new(|cx| {
        //         SelectState::new(
        //             options.clone(), // 👈 SearchableVec<OptCate>
        //             Some(IndexPath::default()),
        //             window,
        //             cx,
        //         )
        //     }));
        // }

        Self {
            data,
            columns: vec![
                Column::new("x", "X").width(40.),
                Column::new("y", "Y").width(40.),
                Column::new("opt_cate", "操作类型").width(80.),
                Column::new("opt_type", "操作方式").width(120.),
                Column::new("win_title", "窗口标题").width(100.),
                Column::new("delay", "延迟").width(80.),
                Column::new("execute_time", "执行次数").width(80.),
            ],
            execute_time_inputs,
            opt_cate_selects,
            opt_type_selects,
            opt_cate_subscriptions,
        }
    }
	fn get_row_opt_cate(&self, row_ix: usize) -> &OptCate {
        &self.data[row_ix].opt_cate
    }

    fn add_row(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
        let new_row_ix = self.data.len();
        self.data.push(MyData {
            x: new_row_ix + 1,
            y: new_row_ix + 1,
            opt_cate: OptCate::鼠标,
            opt_type: "左键单击".to_string(),
            win_title: Some("chrome".to_string()),
            delay: 1,
            execute_time: 1,
        });

        // 给新行创建 Input —— 用传进来的 window
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value("", window, cx);
            state
        });
        self.execute_time_inputs.push(input);

        // 给新行创建 Select —— 用传进来的 window
        let options = SearchableVec::new(vec![OptCate::鼠标, OptCate::键盘]);

        let select = cx.new(|cx| SelectState::new(options, Some(IndexPath::default()), window, cx));
		
        self.opt_cate_selects.push(select);

        let opt_cate_select = self.opt_cate_selects[new_row_ix].clone();
        let subscription = cx.subscribe_in(
            &opt_cate_select,
            window,
            move |table: &mut TableState<MyTableDelegate>, _select, event: &SelectEvent<SearchableVec<OptCate>>, _window, cx: &mut Context<TableState<MyTableDelegate>>| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    table.delegate_mut().data[new_row_ix].opt_cate = match value.as_ref() {
                        "鼠标" => OptCate::鼠标,
                        "键盘" => OptCate::键盘,
                        _ => return,
                    };
                    cx.notify();
                }
            },
        );
        self.opt_cate_subscriptions.push(subscription);

		 
        // 给新行创建 Select —— 用传进来的 window
        let options = SearchableVec::new(vec![  MouseOptType::左键单击,
            MouseOptType::左键双击,
            MouseOptType::滚轮上滑,
            MouseOptType::滚轮下滑,
            MouseOptType::滚轮单击,
            MouseOptType::右键单击,
            MouseOptType::右键双击,]);
        let select: Entity<SelectState<SearchableVec<MouseOptType>>> = cx.new(|cx| SelectState::new(options, Some(IndexPath::default()), window, cx));
        self.opt_type_selects.push(select);

        cx.notify();
    }
}

impl TableDelegate for MyTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.data.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
            .h(px(20.0)) // 👈 表头高度
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgb(0xf8f9fa))
            .w_full()
        
            .text_size(px(13.0))
            .child(self.column(col_ix, cx).name.clone())
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        div().id(("row", row_ix)).h(px(30.0)).p(px(0.))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let row = &self.data[row_ix];
        let col = &self.columns[col_ix];

        let text = match col.key.as_ref() {
            "x" => row.x.to_string().into_any_element(),
            "y" => row.y.to_string().into_any_element(),
            "opt_cate" => div()
                .flex()
                .items_center()
                .rounded_sm()
                .id("opt_cate")
                .child(
                    Select::new(&self.opt_cate_selects[row_ix])
                        .placeholder("选择类型")
                        .small()
                        .w(px(70.0))
                        
						,
                )
                .into_any_element(),
            "opt_type" => {
				match self.get_row_opt_cate(row_ix) {
					OptCate::鼠标 => {
						div()
							.flex()
							.items_center()
							.rounded_sm()
							.child(
								Select::new(&self.opt_type_selects[row_ix])
									.placeholder("操作方式")
									.small()
									.w(px(95.0)),
							)
							.into_any_element()
					}
					OptCate::键盘 => {
						div()
							.flex()
							.items_center()
							.rounded_sm()
							.child(
								Input::new(&self.execute_time_inputs[row_ix]) // 复用 delay 的 input 或单独建一个
									.size_full()
									.w(px(95.0))
									.px_1()
									.text_xs()
									.rounded(px(4.0))
									.py_0()
							)
							.into_any_element()
					}
				}
			},
            "win_title" => row.win_title.clone().unwrap().into_any_element(),
            "delay" => div()
                .flex()
                .items_center()
                .rounded_sm()
                .child(
                    Input::new(&self.execute_time_inputs[row_ix])
                        .size_full()
                        .w(px(50.))
                        .px_1()
                        .text_xs()
                        .rounded(px(4.))
                        .py_0(),
                )
                .into_any_element(),
            "execute_time" => div()
                .flex()
                .items_center()
                .rounded_sm()
                .child(
                    Input::new(&self.execute_time_inputs[row_ix])
                        .size_full()
                        .w(px(70.))
                        .px_1()
                        .text_xs()
                        .rounded(px(4.))
                        .py_0(),
                )
                .into_any_element(),
            _ => "".to_string().into_any_element(),
        };

        div().p(px(0.0)).flex().text_xs().items_center().child(text)
    }
}
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
};

impl Render for IClick {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .text_xs()
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
                                    .w(px(200.0))
                                    .h(px(250.0))
                                    .mr(px(10.0))
                                    .flex()
                                    .flex_col()
                                    .child(
                                        GroupBox::new()
                                            .title("获取坐标方式一")
                                            .title_style(StyleRefinement::default().text_sm())
                                            .h(px(130.0))
                                            .mb(px(10.0))
                                            .child(
                                                div()
                                                    .w_full()
                                                    .text_xs()
                                                    .flex_1()
                                                    .h(px(100.0))
                                                    .border(px(1.0))
                                                    .border_color(cx.theme().border)
                                                    .p(px(5.))
                                                    .flex()
                                                    .flex_col()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .border(px(1.0))
                                                            .border_color(cx.theme().border)
                                                            .w(px(30.))
                                                            .h(px(30.))
                                                            .cursor(CursorStyle::Crosshair)
                                                            .child(
                                                                Icon::new(IconName::Plus)
                                                                    .size_full()
                                                                    .text_color(rgb(0x1f44fa)),
                                                            ),
                                                    )
                                                    .child(
                                                        div().child(
                                                            RadioGroup::horizontal(
                                                                "horizontal-group",
                                                            )
                                                            .text_xs()
                                                            .children(["鼠标事件", "键盘事件"])
                                                            .selected_index(self.opt_cate)
                                                            .on_click(cx.listener(
                                                                |view, index, _, cx| {
                                                                    view.opt_cate = Some(*index);
                                                                    cx.notify();
                                                                },
                                                            )),
                                                        ),
                                                    )
                                                    .child(
                                                        div().child(
                                                            Checkbox::new("hide")
                                                                .label("选择时隐藏本窗口")
                                                                .checked(self.hide_when_choose)
                                                                .on_click({
                                                                    let view = cx.entity().clone();
                                                                    move |_, _window, cx| {
                                                                        view.update(cx, |this, cx| {
                                                                        this.hide_when_choose =
                                                                            !this.hide_when_choose;
                                                                        cx.notify();
                                                                    });
                                                                    }
                                                                }),
                                                        ),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        GroupBox::new()
                                            .title("获取坐标方式二")
                                            .title_style(
                                                StyleRefinement::default().text_sm().h(px(20.0)),
                                            )
                                            .h(px(110.0))
                                            .child(
                                                div()
                                                    .w_full()
                                                    .h(px(80.0))
                                                    .border(px(1.0))
                                                    .border_color(cx.theme().border)
                                                    .flex()
                                                    .p_1()
                                                    .flex_col()
                                                    .justify_between()
                                                    .child(
                                                        Checkbox::new("front_check")
                                                            .label("监听热键")
                                                            .checked(self.watch_hotkey)
                                                            .on_click({
                                                                let view = cx.entity().clone();
                                                                move |_, _window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.watch_hotkey =
                                                                            !this.watch_hotkey;
                                                                        cx.notify();
                                                                    });
                                                                }
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .child(
                                                                div()
                                                                    .child("鼠标事件快捷键："),
                                                            )
                                                            .child(
																Kbd::new(Keystroke::parse("f6").unwrap()).outline().pt_1().pb_1().flex_1()
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .child(
                                                                div()
                                                                    .child("键盘事件快捷键："),
                                                            )
                                                            .child(
																Kbd::new(Keystroke::parse("f7").unwrap()).outline().pt_1().pb_1().flex_1()
                                                            ),
                                                    ),
                                            ),
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
                                                    .justify_between()
                                                
                                                    .child(
														div()
                										.flex()
                                                    .id("set_on_top")
                                                    .cursor(CursorStyle::PointingHand)

                        								.items_center()
														    .on_click({
                                                                let view = cx.entity().clone();
                                                                move |_, _window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.always_on_top =
                                                                            !this.always_on_top;
                                                                        unsafe {
                                                                            let hwnd =
                                                                                GetForegroundWindow(
                                                                                );
                                                                            SetWindowPos(
                                                                                hwnd,
                                                                                if this
                                                                                    .always_on_top
                                                                                {
                                                                                    HWND_TOPMOST
                                                                                } else {
                                                                                    HWND_NOTOPMOST
                                                                                },
                                                                                0,
                                                                                0,
                                                                                0,
                                                                                0,
                                                                                SWP_NOMOVE
                                                                                    | SWP_NOSIZE,
                                                                            )
                                                                            .unwrap();
                                                                        }
                                                                        cx.notify();
                                                                    });
                                                                }
                                                            })
														.child(
															div().text_xs().mr(px(10.)).child("窗口置顶：")
														).child(
														   Checkbox::new("always_on_top")
                                                            .label("")
                                                            .checked(self.always_on_top)
														)
                                                     
                                                    )
                                                    .child(
                                                        Button::new("start_pause")
                                                            .label("开始")
                                                            .w(px(140.0))
                                                            .cursor(CursorStyle::PointingHand)
                                                            .on_click({
																// 	let entity = cx.entity().clone();
                                                               	// 	move |_, _window, cx| {
																// 	   entity.update(cx, |this, cx| {
																// 		let value = this
																// 			.loop_count_input
																// 			.read(cx)
																// 			.value()
																// 			.to_string();

																// 		println!("loop_count = {}", value);
																// 	});
                                                                   
                                                                // }
																let data_table =
                                                                    self.data_table.clone();
																move |_, window, cx| {
															 	data_table.update(
                                                                        cx,
                                                                        |state, cx| {
                                                                            state
                                                                                .delegate_mut()
                                                                                .add_row(
                                                                                    window, cx,
                                                                                );
                                                                        },
                                                                    );
                                                                }

                                                            }),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .flex()
                                                    .justify_between()
                                                    .child(
														div()
                										.flex()
                        								.items_center()
                                                       .cursor(CursorStyle::PointingHand)
                                                    .id("front")
														  .on_click({
                                                                let view = cx.entity().clone();
                                                                move |_, _window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.front_opt =
                                                                            !this.front_opt;
                                                                        cx.notify();
                                                                    });
                                                                }
                                                        })

														.child(
															div().mr(px(10.)).text_xs().child("前台操作：")
														)
														.child( Checkbox::new("front_check")
                                                            .label("")
                                                            .checked(self.front_opt)
                                                          
													)
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .justify_between()
                                                            .w(px(140.0))
                                                            .child(
                                                                Button::new("read_btn")
                                                                    .label("读取")
                                                                    .small()
                                                                    .mr(px(10.0))
                                                                    .cursor(
                                                                        CursorStyle::PointingHand,
                                                                    ).on_click({
																		let entity = cx.entity().clone();
																		move |_, _window, cx| {
																		entity.update(cx, |this, cx| {
																			this.data_table.update(cx, |table_state, _cx| {
																				let delegate = table_state.delegate();
																				for (i, row) in delegate.data.iter().enumerate() {
																					println!(
																						"row {}: x={}, y={}, opt_cate={:?}, opt_type={}, delay={}, execute_time={}", 
																						i,
																						row.x,
																						row.y,
																						row.opt_cate,
																						row.opt_type,
																						row.delay,
																						row.execute_time,
																					);
																				}
																				println!("total rows: {}", delegate.data.len());
																			});
																		});
																	}
																	})
                                                            )
                                                            .child(
                                                                Button::new("save_btn")
                                                                    .label("保存")
                                                                    .small()
                                                                    .cursor(
                                                                        CursorStyle::PointingHand,
                                                                    ),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .child(
                                                                div()
                                                                    .mr(px(10.0))
                                                                    .child("循环次数：")
																	.id("loop_count_tooltip")
																	.tooltip(|window, cx| {
																			Tooltip::new("表格中所有操作的总执行次数，0代表无限次").build(window, cx)
																	}),
                                                            )
                                                            .child(div().w(px(80.0)).child(
                                                                Input::new(&self.loop_count_input).small(),
                                                            )),
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
																	Kbd::new(Keystroke::parse("f10").unwrap()).outline().pt_1().pb_1()
                                                                ),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
														div()
														.flex()
														.items_center()
                                                       .cursor(CursorStyle::PointingHand)
														.id("fuzzy")
														.on_click({
                                                                let view = cx.entity().clone();
                                                                move |_, _window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.fuzzy_click =
                                                                            !this.fuzzy_click;
                                                                        cx.notify();
                                                                    });
                                                                }
                                                            })
														.child(
															div().mr(px(10.)).text_xs().child("模糊点击：")
														).child(  Checkbox::new("fuzzy_click")
                                                            .label("")
                                                            .checked(self.fuzzy_click)
                                                            
														)
                                                      
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
                                                                    Input::new(
                                                                        &self.fuzzy_round_input,
                                                                    ).small()
                                                                    .suffix(div().child("px")),
                                                                ),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .child(
                                                                div()
                                                                    .mr(px(10.0))
                                                                    .child("操作间隔："),
                                                            )
                                                            .child(div().w(px(80.0)).child(
                                                                Input::new(&self.opt_gap_ipt).small().suffix(div().child("ms")),
                                                            )),
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
                                                                    Input::new(
                                                                        &self.loop_gap_ipt,
                                                                    ).small()
                                                                    .suffix(div().child("ms")),
                                                                ),
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
                                                        // Checkbox::new("terms")
                                                        //     .label("")
                                                        //     .checked(false),
														Kbd::new(Keystroke::parse("f8").unwrap()).outline().pt_1().pb_1().min_w(px(100.))
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(200.0))
                            .child(DataTable::new(&self.data_table).with_size(px(20.))),
                    )
            } else if self.active_tab == 1 {
                div().size_full().bg(rgb(0xf6f6f6))
            } else {
                div()
            })
    }
}
use regex::Regex;
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
                                x: gpui::px(560.0),
                                y: gpui::px(550.0),
                            },
                            size: gpui::Size {
                                width: gpui::px(560.0),
                                height: gpui::px(550.0),
                            },
                        })),
                        titlebar: Some(TitlebarOptions {
                            title: Some("iClick-gpui By：吾爱破解：xarzhi".into()),
                            ..Default::default()
                        }),
                        is_resizable: false,
                        ..WindowOptions::default()
                    },
                    |window, cx| {
                        let loop_count_input = cx.new(|cx| InputState::new(window, cx).default_value("1").pattern(Regex::new(r"^[0-9]\d*$").unwrap()));
                        let fuzzy_round_input = cx.new(|cx| InputState::new(window, cx).default_value("0").pattern(Regex::new(r"^[0-9]\d*$").unwrap()));
						  let opt_gap_ipt = cx.new(|cx| InputState::new(window, cx).default_value("20").pattern(Regex::new(r"^[0-9]\d*$").unwrap()));
                        let loop_gap_ipt = cx.new(|cx| InputState::new(window, cx).default_value("0").pattern(Regex::new(r"^[0-9]\d*$").unwrap()));
                      

                        let delegate = MyTableDelegate::new(window, cx);
                        let state = cx.new(|cx| TableState::new(delegate, window, cx));

                        let view = cx.new(|_| IClick {
                            active_tab: 0,
                            tabs: vec![
                                "操作一".to_string(),
                                "操作二".to_string(),
                                "设置".to_string(),
                            ],
                            loop_count_input,
                            fuzzy_round_input,
							loop_gap_ipt,
							opt_gap_ipt,
                            data_table: state,
                            always_on_top: false,
                            front_opt: false,
                            fuzzy_click: false,
                            opt_cate: Some(0),
                            hide_when_choose: true,
                            watch_hotkey: false,
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
