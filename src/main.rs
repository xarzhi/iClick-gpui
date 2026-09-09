// #![windows_subsystem = "windows"]
use gpui::Bounds;
use gpui::SharedString;
use gpui::WindowOptions;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::kbd::Kbd;
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::radio::RadioGroup;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_component::tooltip::Tooltip;
use gpui_component::*;
use gpui_component::{Icon, IconName};
use rdev::{Button as RdevButton, Event, EventType, Key as RdevKey, listen};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::Duration;
use std::vec;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MAPVK_VK_TO_VSC, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, MapVirtualKeyW,
    SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowTextW, IDC_CROSS,
    LoadCursorW, OCR_NORMAL, SPI_SETCURSORS, SPIF_SENDCHANGE, SW_HIDE, SW_SHOW, SendMessageW,
    SetCursorPos, SetForegroundWindow, SetSystemCursor, ShowWindow, SystemParametersInfoW, WM_CHAR,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WindowFromPoint,
};

fn set_selection_cursor(crosshair: bool) {
    unsafe {
        if crosshair {
            if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
                let _ = SetSystemCursor(cursor, OCR_NORMAL);
            }
        } else {
            let _ = SystemParametersInfoW(SPI_SETCURSORS, 0, None, SPIF_SENDCHANGE);
        }
    }
}
fn window_info_at(x: f64, y: f64) -> (String, String) {
    let point = POINT {
        x: x.round() as i32,
        y: y.round() as i32,
    };

    unsafe {
        let hwnd = WindowFromPoint(point);
        if hwnd.0.is_null() {
            return (String::new(), String::new());
        }

        let handle = format!("0x{:X}", hwnd.0 as usize);
        let mut title_buffer = vec![0u16; 512];
        let title_length = GetWindowTextW(hwnd, &mut title_buffer);
        let title = String::from_utf16_lossy(&title_buffer[..title_length as usize]);

        (handle, title)
    }
}

fn cursor_info() -> (f64, f64, String, String) {
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        let _ = GetCursorPos(&mut point);
    }
    let screen_x = point.x as f64;
    let screen_y = point.y as f64;
    let (hwnd_text, title) = window_info_at(screen_x, screen_y);
    let hwnd_value = usize::from_str_radix(hwnd_text.trim_start_matches("0x"), 16)
        .ok()
        .map(|value| HWND(value as *mut _));
    if let Some(hwnd) = hwnd_value {
        unsafe {
            let _ = ScreenToClient(hwnd, &mut point);
        }
    }
    (point.x as f64, point.y as f64, hwnd_text, title)
}

enum HotkeyEvent {
    MouseMove(f64, f64, String, String),
    MouseReleased,
    AddMouse(f64, f64, String, String),
    AddKeyboard(f64, f64, String, String),
    KeyboardInput(usize, String),
    ToggleExecution,
    ExecutionFinished,
}

pub struct IClick {
    active_tab: usize,
    tabs: Vec<String>,
    always_on_top: bool,
    front_opt: bool,
    opt_cate: Option<usize>,
    fuzzy_click: bool,
    hide_when_choose: bool,
    watch_hotkey: bool,
    hotkey_flag: Arc<AtomicBool>,
    execution_flag: Arc<AtomicBool>,
    is_running: bool,
    execution_event_tx: async_channel::Sender<HotkeyEvent>,
    is_choosing: bool,
    choosing_flag: Arc<AtomicBool>,
    hidden_window: Arc<std::sync::Mutex<Option<HWND>>>,
    x_input: Entity<InputState>,
    y_input: Entity<InputState>,
    win_hwnd_input: Entity<InputState>,
    win_title_input: Entity<InputState>,
    loop_count_input: Entity<InputState>,  // 循环次数
    fuzzy_round_input: Entity<InputState>, // 模糊范围
    loop_gap_ipt: Entity<InputState>,      // 循环间隔
    opt_gap_ipt: Entity<InputState>,       // 循环间隔
    _input_subscriptions: Vec<Subscription>,
    data_table: Entity<TableState<MyTableDelegate>>,
}

fn subscribe_select_all(input: &Entity<InputState>, cx: &mut App) -> Subscription {
    cx.subscribe(input, move |state, event: &InputEvent, cx| {
        if let InputEvent::Focus = event {
            let length = state.read(cx).value().len();
            state.update(cx, |input, cx| {
                input.set_selected_range(0..length, cx);
            });
        }
    })
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

static MOUSE_OPT_TYPE_LABELS: LazyLock<[SharedString; 7]> = LazyLock::new(|| {
    [
        "左键单击".into(),
        "左键双击".into(),
        "滚轮上滑".into(),
        "滚轮下滑".into(),
        "滚轮单击".into(),
        "右键单击".into(),
        "右键双击".into(),
    ]
});

impl MouseOptType {
    fn index(&self) -> usize {
        match self {
            Self::左键单击 => 0,
            Self::左键双击 => 1,
            Self::滚轮上滑 => 2,
            Self::滚轮下滑 => 3,
            Self::滚轮单击 => 4,
            Self::右键单击 => 5,
            Self::右键双击 => 6,
        }
    }
}

impl SelectItem for MouseOptType {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        MOUSE_OPT_TYPE_LABELS[self.index()].clone()
    }

    fn value(&self) -> &Self::Value {
        &MOUSE_OPT_TYPE_LABELS[self.index()]
    }
}
#[derive(Clone, Debug)]

struct MyData {
    x: usize,
    y: usize,
    win_hwnd: String,
    opt_cate: OptCate,
    opt_type: String,
    win_title: Option<String>,
    delay: usize,
    execute_time: usize,
}

static OPT_CATE_LABELS: LazyLock<[SharedString; 2]> =
    LazyLock::new(|| ["鼠标".into(), "键盘".into()]);

impl OptCate {
    fn index(&self) -> usize {
        match self {
            Self::鼠标 => 0,
            Self::键盘 => 1,
        }
    }
}

impl SelectItem for OptCate {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        OPT_CATE_LABELS[self.index()].clone()
    }

    fn value(&self) -> &Self::Value {
        &OPT_CATE_LABELS[self.index()]
    }
}

struct MyTableDelegate {
    data: Vec<MyData>,
    columns: Vec<Column>,
    delay_inputs: Vec<Entity<InputState>>,
    execute_time_inputs: Vec<Entity<InputState>>,
    opt_type_inputs: Vec<Entity<InputState>>,
    opt_cate_selects: Vec<Entity<SelectState<SearchableVec<OptCate>>>>,
    opt_type_selects: Vec<Entity<SelectState<SearchableVec<MouseOptType>>>>,
    delay_subscriptions: Vec<Subscription>,
    execute_time_subscriptions: Vec<Subscription>,
    opt_cate_subscriptions: Vec<Subscription>,
    opt_type_subscriptions: Vec<Subscription>,
    focused_keyboard_row: Arc<std::sync::Mutex<Option<usize>>>,
    expected_keyboard_value: Arc<std::sync::Mutex<Option<(usize, String)>>>,
}

impl MyTableDelegate {
    fn new(
        _window: &mut Window,
        _cx: &mut App,
        focused_keyboard_row: Arc<std::sync::Mutex<Option<usize>>>,
        expected_keyboard_value: Arc<std::sync::Mutex<Option<(usize, String)>>>,
    ) -> Self {
        let data: Vec<MyData> = vec![];
        let delay_inputs: Vec<Entity<InputState>> = vec![];
        let execute_time_inputs: Vec<Entity<InputState>> = vec![];
        let opt_type_inputs: Vec<Entity<InputState>> = vec![];
        let opt_cate_selects: Vec<Entity<SelectState<SearchableVec<OptCate>>>> = vec![];
        let opt_type_selects: Vec<Entity<SelectState<SearchableVec<MouseOptType>>>> = vec![];
        let delay_subscriptions: Vec<Subscription> = vec![];
        let execute_time_subscriptions: Vec<Subscription> = vec![];
        let opt_cate_subscriptions: Vec<Subscription> = vec![];
        let opt_type_subscriptions: Vec<Subscription> = vec![];

        Self {
            data,
            columns: vec![
                Column::new("x", "X").width(40.),
                Column::new("y", "Y").width(40.),
                Column::new("opt_cate", "操作类型").width(80.),
                Column::new("opt_type", "操作方式").width(120.),
                Column::new("win_title", "窗口标题").width(100.),
                Column::new("delay", "延迟(ms)").width(80.),
                Column::new("execute_time", "执行次数").width(80.),
            ],
            delay_inputs,
            execute_time_inputs,
            opt_type_inputs,
            opt_cate_selects,
            opt_type_selects,
            delay_subscriptions,
            execute_time_subscriptions,
            opt_cate_subscriptions,
            opt_type_subscriptions,
            focused_keyboard_row,
            expected_keyboard_value,
        }
    }
    fn get_row_opt_cate(&self, row_ix: usize) -> &OptCate {
        &self.data[row_ix].opt_cate
    }

    fn add_row(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>, data: MyData) {
        let new_row_ix = self.data.len();
        self.data.push(data.clone());

        let delay_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).pattern(Regex::new(r"^[0-9]\d*$").unwrap());
            state.set_value(&data.delay.to_string(), window, cx);
            state
        });
        self.delay_inputs.push(delay_input.clone());

        let subscription = cx.subscribe_in(
            &delay_input,
            window,
            move |table: &mut TableState<MyTableDelegate>,
                  input,
                  event: &InputEvent,
                  _window,
                  cx: &mut Context<TableState<MyTableDelegate>>| {
                if let InputEvent::Focus = event {
                    let length = input.read(cx).value().len();
                    input.update(cx, |state, cx| {
                        state.set_selected_range(0..length, cx);
                    });
                } else if let InputEvent::Change = event {
                    if let Ok(value) = input.read(cx).value().parse::<usize>() {
                        let Some(row_ix) = table
                            .delegate()
                            .delay_inputs
                            .iter()
                            .position(|candidate| candidate == input)
                        else {
                            return;
                        };
                        table.delegate_mut().data[row_ix].delay = value;
                        cx.notify();
                    }
                }
            },
        );
        self.delay_subscriptions.push(subscription);

        let execute_time_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).pattern(Regex::new(r"^[0-9]\d*$").unwrap());
            state.set_value(&data.execute_time.to_string(), window, cx);
            state
        });
        self.execute_time_inputs.push(execute_time_input.clone());

        let subscription = cx.subscribe_in(
            &execute_time_input,
            window,
            move |table: &mut TableState<MyTableDelegate>,
                  input,
                  event: &InputEvent,
                  _window,
                  cx: &mut Context<TableState<MyTableDelegate>>| {
                if let InputEvent::Focus = event {
                    let length = input.read(cx).value().len();
                    input.update(cx, |state, cx| {
                        state.set_selected_range(0..length, cx);
                    });
                } else if let InputEvent::Change = event {
                    if let Ok(value) = input.read(cx).value().parse::<usize>() {
                        let Some(row_ix) = table
                            .delegate()
                            .execute_time_inputs
                            .iter()
                            .position(|candidate| candidate == input)
                        else {
                            return;
                        };
                        table.delegate_mut().data[row_ix].execute_time = value;
                        cx.notify();
                    }
                }
            },
        );
        self.execute_time_subscriptions.push(subscription);

        let opt_type_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(&data.opt_type, window, cx);
            state
        });
        self.opt_type_inputs.push(opt_type_input.clone());

        let subscription = cx.subscribe_in(
            &opt_type_input,
            window,
            move |table: &mut TableState<MyTableDelegate>,
                  input,
                  event: &InputEvent,
                  _window,
                  cx: &mut Context<TableState<MyTableDelegate>>| {
                match event {
                    InputEvent::Focus => {
                        let length = input.read(cx).value().len();
                        input.update(cx, |state, cx| {
                            state.set_selected_range(0..length, cx);
                        });
                        let Some(row_ix) = table
                            .delegate()
                            .opt_type_inputs
                            .iter()
                            .position(|candidate| candidate == input)
                        else {
                            return;
                        };
                        *table.delegate().focused_keyboard_row.lock().unwrap() = Some(row_ix);
                    }
                    InputEvent::Blur => {
                        let Some(row_ix) = table
                            .delegate()
                            .opt_type_inputs
                            .iter()
                            .position(|candidate| candidate == input)
                        else {
                            return;
                        };
                        let mut focused_row = table.delegate().focused_keyboard_row.lock().unwrap();
                        if *focused_row == Some(row_ix) {
                            *focused_row = None;
                        }
                    }
                    InputEvent::Change => {
                        let Some(row_ix) = table
                            .delegate()
                            .opt_type_inputs
                            .iter()
                            .position(|candidate| candidate == input)
                        else {
                            return;
                        };
                        let expected_value = table
                            .delegate()
                            .expected_keyboard_value
                            .lock()
                            .unwrap()
                            .clone()
                            .filter(|(expected_row_ix, _)| *expected_row_ix == row_ix)
                            .map(|(_, value)| value);
                        let value =
                            expected_value.unwrap_or_else(|| input.read(cx).value().to_string());
                        if input.read(cx).value().as_ref() != value.as_str() {
                            input.update(cx, |state, cx| {
                                state.set_value(&value, _window, cx);
                            });
                        }
                        table.delegate_mut().data[row_ix].opt_type = value;
                        cx.notify();
                    }
                    _ => {}
                }
            },
        );
        self.opt_type_subscriptions.push(subscription);

        // 给新行创建 Select —— 用传进来的 window
        let options = SearchableVec::new(vec![OptCate::鼠标, OptCate::键盘]);

        let opt_cate_index = match data.opt_cate {
            OptCate::鼠标 => 0,
            OptCate::键盘 => 1,
        };
        let select = cx
            .new(|cx| SelectState::new(options, Some(IndexPath::new(opt_cate_index)), window, cx));

        self.opt_cate_selects.push(select);

        let opt_cate_select = self.opt_cate_selects[new_row_ix].clone();
        let subscription = cx.subscribe_in(
            &opt_cate_select,
            window,
            move |table: &mut TableState<MyTableDelegate>,
                  select,
                  event: &SelectEvent<SearchableVec<OptCate>>,
                  _window,
                  cx: &mut Context<TableState<MyTableDelegate>>| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    let Some(row_ix) = table
                        .delegate()
                        .opt_cate_selects
                        .iter()
                        .position(|candidate| candidate == select)
                    else {
                        return;
                    };
                    table.delegate_mut().data[row_ix].opt_cate = match value.as_ref() {
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
        let options = SearchableVec::new(vec![
            MouseOptType::左键单击,
            MouseOptType::左键双击,
            MouseOptType::滚轮上滑,
            MouseOptType::滚轮下滑,
            MouseOptType::滚轮单击,
            MouseOptType::右键单击,
            MouseOptType::右键双击,
        ]);
        let select: Entity<SelectState<SearchableVec<MouseOptType>>> =
            cx.new(|cx| SelectState::new(options, Some(IndexPath::default()), window, cx));
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

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        let table = cx.entity().clone();
        menu.item(PopupMenuItem::new("删除本行").on_click(move |_, _, cx| {
            table.update(cx, |table, cx| {
                table.delegate_mut().remove_row(row_ix);
                cx.notify();
            });
        }))
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
                        .w(px(70.0)),
                )
                .into_any_element(),
            "opt_type" => match self.get_row_opt_cate(row_ix) {
                OptCate::鼠标 => div()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .child(
                        Select::new(&self.opt_type_selects[row_ix])
                            .placeholder("操作方式")
                            .small()
                            .w(px(95.0)),
                    )
                    .into_any_element(),
                OptCate::键盘 => div()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .child(
                        Input::new(&self.opt_type_inputs[row_ix])
                            .size_full()
                            .w(px(95.0))
                            .px_1()
                            .text_xs()
                            .rounded(px(4.0))
                            .py_0(),
                    )
                    .into_any_element(),
            },
            "win_title" => row.win_title.clone().unwrap().into_any_element(),
            "delay" => div()
                .flex()
                .items_center()
                .rounded_sm()
                .child(
                    Input::new(&self.delay_inputs[row_ix])
                        .size_full()
                        .w(px(65.))
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

impl MyTableDelegate {
    #[allow(unused_must_use)]
    fn remove_row(&mut self, row_ix: usize) {
        if row_ix >= self.data.len() {
            return;
        }

        self.data.remove(row_ix);
        self.delay_inputs.remove(row_ix);
        self.execute_time_inputs.remove(row_ix);
        self.opt_type_inputs.remove(row_ix);
        self.opt_cate_selects.remove(row_ix);
        self.opt_type_selects.remove(row_ix);
        let _ = self.delay_subscriptions.remove(row_ix);
        let _ = self.execute_time_subscriptions.remove(row_ix);
        let _ = self.opt_cate_subscriptions.remove(row_ix);
        let _ = self.opt_type_subscriptions.remove(row_ix);

        let mut focused_row = self.focused_keyboard_row.lock().unwrap();
        if *focused_row == Some(row_ix) {
            *focused_row = None;
        } else if focused_row.is_some_and(|focused| focused > row_ix) {
            *focused_row = focused_row.map(|focused| focused - 1);
        }

        let mut expected_value = self.expected_keyboard_value.lock().unwrap();
        if expected_value
            .as_ref()
            .is_some_and(|(focused, _)| *focused == row_ix)
        {
            *expected_value = None;
        } else if let Some((focused, value)) = expected_value.take() {
            *expected_value = Some((
                if focused > row_ix {
                    focused - 1
                } else {
                    focused
                },
                value,
            ));
        }
    }
}
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
};

impl IClick {
    fn toggle_execution(&mut self, cx: &mut Context<Self>) {
        if self.is_running {
            self.is_running = false;
            self.execution_flag.store(false, Ordering::Release);
            cx.notify();
            return;
        }

        let rows = self.data_table.read(cx).delegate().data.clone();
        if rows.is_empty() {
            return;
        }
        let loop_count = self
            .loop_count_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(1);
        let front_opt = self.front_opt;

        self.is_running = true;
        self.execution_flag.store(true, Ordering::Release);
        let execution_flag = self.execution_flag.clone();
        let execution_event_tx = self.execution_event_tx.clone();
        thread::spawn(move || {
            let mut completed_loops = 0;
            while loop_count == 0 || completed_loops < loop_count {
                for row in &rows {
                    if !execution_flag.load(Ordering::Acquire) {
                        break;
                    }

                    let Some(hwnd) = parse_hwnd(&row.win_hwnd) else {
                        continue;
                    };
                    for _ in 0..row.execute_time.max(1) {
                        if !execution_flag.load(Ordering::Acquire) {
                            break;
                        }

                        send_row_input(hwnd, row, front_opt);
                        if row.delay > 0 {
                            thread::sleep(Duration::from_millis(row.delay as u64));
                        }
                    }
                }
                if !execution_flag.load(Ordering::Acquire) {
                    break;
                }
                completed_loops += 1;
            }
            execution_flag.store(false, Ordering::Release);
            let _ = execution_event_tx.try_send(HotkeyEvent::ExecutionFinished);
        });
        cx.notify();
    }

    fn hide_for_selection(&mut self) {
        set_selection_cursor(true);
        if !self.hide_when_choose {
            return;
        }

        unsafe {
            let hwnd = GetForegroundWindow();
            if !hwnd.0.is_null() {
                let _ = ShowWindow(hwnd, SW_HIDE);
                *self.hidden_window.lock().unwrap() = Some(hwnd);
            }
        }
    }

    fn show_after_selection(&mut self) {
        set_selection_cursor(false);
        if let Some(hwnd) = self.hidden_window.lock().unwrap().take() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
        }
    }

    fn add_hotkey_row(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        x: f64,
        y: f64,
        hwnd: String,
        title: String,
        opt_cate: OptCate,
    ) {
        self.x_input.update(cx, |input, cx| {
            input.set_value(&(x.round() as usize).to_string(), window, cx);
        });
        self.y_input.update(cx, |input, cx| {
            input.set_value(&(y.round() as usize).to_string(), window, cx);
        });
        self.win_title_input.update(cx, |input, cx| {
            input.set_value(&title, window, cx);
        });

        let opt_type = match opt_cate {
            OptCate::鼠标 => "左键单击".to_string(),
            OptCate::键盘 => String::new(),
        };
        let data = MyData {
            x: x.round() as usize,
            y: y.round() as usize,
            win_hwnd: hwnd,
            opt_cate,
            opt_type,
            win_title: Some(title),
            delay: 0,
            execute_time: 1,
        };
        self.data_table.update(cx, |table, cx| {
            table.delegate_mut().add_row(window, cx, data);
        });
        cx.notify();
    }

    fn finish_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_choosing {
            return;
        }

        self.is_choosing = false;
        self.choosing_flag.store(false, Ordering::Release);
        self.show_after_selection();

        let x = self.x_input.read(cx).value().parse::<usize>().unwrap_or(0);
        let y = self.y_input.read(cx).value().parse::<usize>().unwrap_or(0);
        let win_title = self.win_title_input.read(cx).value().to_string();
        let win_hwnd = self.win_hwnd_input.read(cx).value().to_string();
        let opt_cate = if self.opt_cate == Some(1) {
            OptCate::键盘
        } else {
            OptCate::鼠标
        };
        let opt_type = match opt_cate {
            OptCate::鼠标 => "左键单击".to_string(),
            OptCate::键盘 => String::new(),
        };

        let data = MyData {
            x,
            y,
            win_hwnd,
            opt_cate,
            opt_type,
            win_title: Some(win_title),
            delay: 0,
            execute_time: 1,
        };

        self.data_table.update(cx, |table, cx| {
            table.delegate_mut().add_row(window, cx, data);
        });
        cx.notify();
    }
}

fn parse_hwnd(value: &str) -> Option<HWND> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    usize::from_str_radix(value, 16)
        .ok()
        .map(|value| HWND(value as *mut std::ffi::c_void))
}

fn keyboard_key_label(key: &RdevKey, shift_pressed: bool) -> String {
    let label = match key {
        RdevKey::KeyA => "A",
        RdevKey::KeyB => "B",
        RdevKey::KeyC => "C",
        RdevKey::KeyD => "D",
        RdevKey::KeyE => "E",
        RdevKey::KeyF => "F",
        RdevKey::KeyG => "G",
        RdevKey::KeyH => "H",
        RdevKey::KeyI => "I",
        RdevKey::KeyJ => "J",
        RdevKey::KeyK => "K",
        RdevKey::KeyL => "L",
        RdevKey::KeyM => "M",
        RdevKey::KeyN => "N",
        RdevKey::KeyO => "O",
        RdevKey::KeyP => "P",
        RdevKey::KeyQ => "Q",
        RdevKey::KeyR => "R",
        RdevKey::KeyS => "S",
        RdevKey::KeyT => "T",
        RdevKey::KeyU => "U",
        RdevKey::KeyV => "V",
        RdevKey::KeyW => "W",
        RdevKey::KeyX => "X",
        RdevKey::KeyY => "Y",
        RdevKey::KeyZ => "Z",
        RdevKey::Num0 => "0",
        RdevKey::Num1 => "1",
        RdevKey::Num2 => "2",
        RdevKey::Num3 => "3",
        RdevKey::Num4 => "4",
        RdevKey::Num5 => "5",
        RdevKey::Num6 => "6",
        RdevKey::Num7 => "7",
        RdevKey::Num8 => "8",
        RdevKey::Num9 => "9",
        RdevKey::F1 => "F1",
        RdevKey::F2 => "F2",
        RdevKey::F3 => "F3",
        RdevKey::F4 => "F4",
        RdevKey::F5 => "F5",
        RdevKey::F6 => "F6",
        RdevKey::F7 => "F7",
        RdevKey::F8 => "F8",
        RdevKey::F9 => "F9",
        RdevKey::F10 => "F10",
        RdevKey::F11 => "F11",
        RdevKey::F12 => "F12",
        RdevKey::Backspace => "BACKSPACE",
        RdevKey::Delete => "DELETE",
        RdevKey::Return => "ENTER",
        RdevKey::Escape => "ESCAPE",
        RdevKey::Space => "SPACE",
        RdevKey::Tab => "TAB",
        RdevKey::LeftArrow => "LEFT",
        RdevKey::RightArrow => "RIGHT",
        RdevKey::UpArrow => "UP",
        RdevKey::DownArrow => "DOWN",
        RdevKey::Home => "HOME",
        RdevKey::End => "END",
        RdevKey::PageUp => "PAGEUP",
        RdevKey::PageDown => "PAGEDOWN",
        RdevKey::Insert => "INSERT",
        RdevKey::PrintScreen => "PRINTSCREEN",
        RdevKey::ScrollLock => "SCROLLLOCK",
        RdevKey::Pause => "PAUSE",
        RdevKey::NumLock => "NUMLOCK",
        RdevKey::BackQuote => "BackQuote",
        RdevKey::Minus | RdevKey::KpMinus => "MINUS",
        RdevKey::Equal | RdevKey::KpPlus => "EQUAL",
        RdevKey::LeftBracket => "LEFTBRACKET",
        RdevKey::RightBracket => "RIGHTBRACKET",
        RdevKey::SemiColon => "SEMICOLON",
        RdevKey::Quote => "QUOTE",
        RdevKey::BackSlash | RdevKey::IntlBackslash => "BACKSLASH",
        RdevKey::Comma => "COMMA",
        RdevKey::Dot | RdevKey::KpDelete => "DOT",
        RdevKey::Slash | RdevKey::KpDivide => "SLASH",
        RdevKey::KpReturn => "ENTER",
        RdevKey::KpMultiply => "MULTIPLY",
        RdevKey::Kp0 => "0",
        RdevKey::Kp1 => "1",
        RdevKey::Kp2 => "2",
        RdevKey::Kp3 => "3",
        RdevKey::Kp4 => "4",
        RdevKey::Kp5 => "5",
        RdevKey::Kp6 => "6",
        RdevKey::Kp7 => "7",
        RdevKey::Kp8 => "8",
        RdevKey::Kp9 => "9",
        RdevKey::ShiftLeft | RdevKey::ShiftRight => "SHIFT",
        RdevKey::ControlLeft | RdevKey::ControlRight => "CTRL",
        RdevKey::Alt | RdevKey::AltGr => "ALT",
        RdevKey::MetaLeft | RdevKey::MetaRight => "WIN",
        _ => return format!("{key:?}").to_uppercase(),
    };
    if !shift_pressed && label.len() == 1 {
        label.to_ascii_lowercase()
    } else {
        label.to_string()
    }
}

fn keyboard_virtual_key(value: &str) -> Option<usize> {
    let value = value.trim().to_uppercase();
    match value.as_str() {
        "BACKSPACE" => Some(0x08),
        "TAB" => Some(0x09),
        "ENTER" => Some(0x0D),
        "SHIFT" => Some(0x10),
        "CTRL" => Some(0x11),
        "ALT" => Some(0x12),
        "PAUSE" => Some(0x13),
        "CAPSLOCK" => Some(0x14),
        "ESCAPE" => Some(0x1B),
        "SPACE" => Some(0x20),
        "PAGEUP" => Some(0x21),
        "PAGEDOWN" => Some(0x22),
        "END" => Some(0x23),
        "HOME" => Some(0x24),
        "LEFT" => Some(0x25),
        "UP" => Some(0x26),
        "RIGHT" => Some(0x27),
        "DOWN" => Some(0x28),
        "INSERT" => Some(0x2D),
        "DELETE" => Some(0x2E),
        "WIN" => Some(0x5B),
        "PRINTSCREEN" => Some(0x2C),
        "SCROLLLOCK" => Some(0x91),
        "NUMLOCK" => Some(0x90),
        "BACKQUOTE" => Some(0xC0),
        "MINUS" => Some(0xBD),
        "EQUAL" => Some(0xBB),
        "LEFTBRACKET" => Some(0xDB),
        "RIGHTBRACKET" => Some(0xDD),
        "SEMICOLON" => Some(0xBA),
        "QUOTE" => Some(0xDE),
        "BACKSLASH" => Some(0xDC),
        "COMMA" => Some(0xBC),
        "DOT" => Some(0xBE),
        "SLASH" => Some(0xBF),
        "MULTIPLY" => Some(0x6A),
        _ if value.len() == 1 && value.as_bytes()[0].is_ascii_alphanumeric() => {
            Some(value.as_bytes()[0] as usize)
        }
        _ if value.len() >= 2 && value.starts_with('F') => value[1..]
            .parse::<usize>()
            .ok()
            .filter(|number| (1..=24).contains(number))
            .map(|number| 0x70 + number - 1),
        _ => None,
    }
}

fn send_input_key(hwnd: HWND, key: &str) {
    unsafe {
        let top_level = GetAncestor(hwnd, GA_ROOT);
        let _ = SetForegroundWindow(if top_level.0.is_null() {
            hwnd
        } else {
            top_level
        });
    }
    let is_character = key.chars().count() == 1;
    let inputs = if !is_character {
        let Some(vk) = keyboard_virtual_key(key) else {
            return;
        };
        vec![
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(vk as u16),
                        ..Default::default()
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(vk as u16),
                        dwFlags: KEYEVENTF_KEYUP,
                        ..Default::default()
                    },
                },
            },
        ]
    } else {
        key.encode_utf16()
            .flat_map(|unit| {
                [
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: unit,
                                dwFlags: KEYEVENTF_UNICODE,
                                ..Default::default()
                            },
                        },
                    },
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: unit,
                                dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                                ..Default::default()
                            },
                        },
                    },
                ]
            })
            .collect()
    };
    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn send_input_mouse(hwnd: HWND, row: &MyData) {
    unsafe {
        let mut point = POINT {
            x: row.x as i32,
            y: row.y as i32,
        };
        let _ = windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut point);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetCursorPos(point.x, point.y);
        let inputs = [
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dwFlags: MOUSEEVENTF_LEFTDOWN,
                        ..Default::default()
                    },
                },
            },
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dwFlags: MOUSEEVENTF_LEFTUP,
                        ..Default::default()
                    },
                },
            },
        ];
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn keyboard_message_lparam(vk: usize, key_up: bool) -> LPARAM {
    let scan_code = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as isize;
    let extended = matches!(
        vk,
        0x21..=0x2E | 0x5B..=0x5C | 0x6F | 0x90..=0x91
    );
    let mut value = 1isize | (scan_code << 16);
    if extended {
        value |= 1 << 24;
    }
    if key_up {
        value |= 0xC000_0000u32 as isize;
    }
    LPARAM(value)
}

fn send_row_input(hwnd: HWND, row: &MyData, front_opt: bool) {
    unsafe {
        match row.opt_cate {
            OptCate::鼠标 => {
                if front_opt {
                    send_input_mouse(hwnd, row);
                    return;
                }
                let x = (row.x as u32) & 0xffff;
                let y = (row.y as u32) & 0xffff;
                let lparam = LPARAM(((y << 16) | x) as isize);
                let _ = SendMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(1), lparam);
                let _ = SendMessageW(hwnd, WM_LBUTTONUP, WPARAM(0), lparam);
            }
            OptCate::键盘 => {
                if front_opt {
                    send_input_key(hwnd, &row.opt_type);
                } else if row.opt_type.chars().count() == 1 {
                    for unit in row.opt_type.encode_utf16() {
                        let _ = SendMessageW(hwnd, WM_CHAR, WPARAM(unit as usize), LPARAM(0));
                    }
                } else if let Some(key) = keyboard_virtual_key(&row.opt_type) {
                    let wparam = WPARAM(key);
                    let _ = SendMessageW(
                        hwnd,
                        WM_KEYDOWN,
                        wparam,
                        keyboard_message_lparam(key, false),
                    );
                    if key == 0x0D {
                        let _ = SendMessageW(hwnd, WM_CHAR, WPARAM(0x0D), LPARAM(0));
                    }
                    let _ =
                        SendMessageW(hwnd, WM_KEYUP, wparam, keyboard_message_lparam(key, true));
                } else {
                    for unit in row.opt_type.encode_utf16() {
                        let _ = SendMessageW(hwnd, WM_CHAR, WPARAM(unit as usize), LPARAM(0));
                    }
                }
            }
        }
    }
}

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
                                                            .flex()
                                                            .items_center()
                                                            .child(
                                                                div()
                                                                    .border(px(1.0))
                                                                    .border_color(cx.theme().border)
                                                                    .w(px(30.))
                                                                    .h(px(30.))
                                                                    .cursor(CursorStyle::Crosshair)
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        cx.listener(|view, _, _, cx| {
                                                                            view.is_choosing = true;
                                                                            view.choosing_flag.store(true, Ordering::Release);
                                                                            view.hide_for_selection();
                                                                            cx.notify();
                                                                        }),
                                                                    )
                                                                    .on_mouse_up(
                                                                        MouseButton::Left,
                                                                        cx.listener(|view, _, window, cx| {
                                                                            view.finish_selection(window, cx);
                                                                        }),
                                                                    )
                                                                    .on_mouse_up_out(
                                                                        MouseButton::Left,
                                                                        cx.listener(|view, _, window, cx| {
                                                                            view.finish_selection(window, cx);
                                                                        }),
                                                                    )
                                                                    .child(
                                                                        Icon::new(IconName::Plus)
                                                                            .size_full()
                                                                            .text_color(rgb(0x1f44fa)),
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .ml(px(8.0))
                                                                    .child(self.is_choosing.to_string()),
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
                                                                        this.hotkey_flag.store(
                                                                            this.watch_hotkey,
                                                                            Ordering::Release,
                                                                        );
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
                                                            .label(if self.is_running {
                                                                "暂停"
                                                            } else {
                                                                "开始"
                                                            })
                                                            .w(px(140.0))
                                                            .cursor(CursorStyle::PointingHand)
                                                            .on_click({
                                                                let view = cx.entity().clone();
                                                                move |_, _window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.toggle_execution(cx);
                                                                    });
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
														Kbd::new(Keystroke::parse("f8").unwrap()).outline().pt_1().pb_1().min_w(px(100.))
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
						div()
						.mb(px(10.))
						.flex()
						.justify_between()
						.items_center()
						.child(
						    div()
							.flex()
							.items_center()
    						.mr(px(10.))
							.child(
								div()
									.mr(px(10.0))
									.child("X：")
									.id("x")

							)
							.child(div().child(
                                                Input::new(&self.x_input).small().w(px(55.)),
							)),
						)
						.child(
						    div()
							.flex()
							.items_center()
    						.mr(px(10.))

							.child(
								div()
									.mr(px(10.0))
									.child("Y：")
									.id("y")

							)
							.child(div().child(
                                                Input::new(&self.y_input).small().w(px(55.)),
							)),
						)
						.child(
						    div()
							.flex()
							.flex_1()
							.items_center()
    						.mr(px(10.))

							.child(
								div()
									.mr(px(10.0))
									.child("窗口句柄：")
									.id("win_hwnd")

							)
							.child(div().flex_1().child(
                                                Input::new(&self.win_hwnd_input).small(),
							)),
						)
						.child(
						    div()
							.flex()
							.items_center()
							.flex_1()
							.child(
								div()
									.mr(px(10.0))
									.child("窗口标题：")
									.id("win_title")
							)
							.child(div().flex_1().child(
                                                Input::new(&self.win_title_input).small(),
							))
						)
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
                        let loop_count_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("1")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let fuzzy_round_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("0")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let opt_gap_ipt = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("20")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let loop_gap_ipt = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("0")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let x_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("0")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let y_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("0")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let win_hwnd_input = cx.new(|cx| InputState::new(window, cx));
                        let win_title_input = cx.new(|cx| InputState::new(window, cx));
                        let input_subscriptions = vec![
                            subscribe_select_all(&loop_count_input, cx),
                            subscribe_select_all(&fuzzy_round_input, cx),
                            subscribe_select_all(&opt_gap_ipt, cx),
                            subscribe_select_all(&loop_gap_ipt, cx),
                            subscribe_select_all(&x_input, cx),
                            subscribe_select_all(&y_input, cx),
                            subscribe_select_all(&win_hwnd_input, cx),
                            subscribe_select_all(&win_title_input, cx),
                        ];

                        let focused_keyboard_row = Arc::new(std::sync::Mutex::new(None::<usize>));
                        let expected_keyboard_value =
                            Arc::new(std::sync::Mutex::new(None::<(usize, String)>));
                        let delegate = MyTableDelegate::new(
                            window,
                            cx,
                            focused_keyboard_row.clone(),
                            expected_keyboard_value.clone(),
                        );
                        let state = cx.new(|cx| TableState::new(delegate, window, cx));
                        let choosing_flag = Arc::new(AtomicBool::new(false));
                        let hotkey_flag = Arc::new(AtomicBool::new(false));
                        let execution_flag = Arc::new(AtomicBool::new(false));
                        let (mouse_tx, mouse_rx) = async_channel::unbounded::<HotkeyEvent>();

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
                            _input_subscriptions: input_subscriptions,
                            data_table: state,
                            always_on_top: false,
                            front_opt: false,
                            fuzzy_click: false,
                            opt_cate: Some(0),
                            hide_when_choose: true,
                            watch_hotkey: false,
                            hotkey_flag: hotkey_flag.clone(),
                            execution_flag: execution_flag.clone(),
                            is_running: false,
                            execution_event_tx: mouse_tx.clone(),
                            is_choosing: false,
                            choosing_flag: choosing_flag.clone(),
                            hidden_window: Arc::new(std::sync::Mutex::new(None)),
                            x_input,
                            y_input,
                            win_hwnd_input,
                            win_title_input,
                        });
                        let listener_flag = choosing_flag.clone();
                        let hotkey_listener_flag = hotkey_flag.clone();
                        let keyboard_input_row = focused_keyboard_row.clone();
                        std::thread::spawn(move || {
                            let mut shift_pressed = false;
                            let mut caps_lock = false;
                            let callback = move |event: Event| {
                                if listener_flag.load(Ordering::Acquire)
                                    && let EventType::MouseMove { x: _, y: _ } = event.event_type
                                {
                                    let (relative_x, relative_y, hwnd, title) = cursor_info();
                                    let _ = mouse_tx.try_send(HotkeyEvent::MouseMove(
                                        relative_x, relative_y, hwnd, title,
                                    ));
                                } else if listener_flag.load(Ordering::Acquire)
                                    && let EventType::ButtonRelease(RdevButton::Left) =
                                        event.event_type
                                {
                                    set_selection_cursor(false);
                                    let _ = mouse_tx.try_send(HotkeyEvent::MouseReleased);
                                } else if let EventType::KeyRelease(key) = event.event_type {
                                    if matches!(key, RdevKey::ShiftLeft | RdevKey::ShiftRight) {
                                        shift_pressed = false;
                                    }
                                } else if let EventType::KeyPress(key) = event.event_type {
                                    if matches!(key, RdevKey::ShiftLeft | RdevKey::ShiftRight) {
                                        shift_pressed = true;
                                    }
                                    if key == RdevKey::CapsLock {
                                        caps_lock = !caps_lock;
                                    }
                                    if let Some(row_ix) = *keyboard_input_row.lock().unwrap() {
                                        let _ = mouse_tx.try_send(HotkeyEvent::KeyboardInput(
                                            row_ix,
                                            keyboard_key_label(&key, shift_pressed ^ caps_lock),
                                        ));
                                        return;
                                    }
                                    if key == RdevKey::F8 {
                                        let _ = mouse_tx.try_send(HotkeyEvent::ToggleExecution);
                                        return;
                                    }
                                    if !hotkey_listener_flag.load(Ordering::Acquire) {
                                        return;
                                    }
                                    let (x, y, hwnd, title) = cursor_info();
                                    match key {
                                        RdevKey::F6 => {
                                            let _ = mouse_tx
                                                .try_send(HotkeyEvent::AddMouse(x, y, hwnd, title));
                                        }
                                        RdevKey::F7 => {
                                            let _ = mouse_tx.try_send(HotkeyEvent::AddKeyboard(
                                                x, y, hwnd, title,
                                            ));
                                        }
                                        _ => {}
                                    }
                                }
                            };
                            if let Err(error) = listen(callback) {
                                eprintln!("rdev mouse listener stopped: {error:?}");
                            }
                        });

                        let mouse_view = view.clone();
                        window
                            .spawn(cx, async move |window_cx| {
                                while let Ok(event) = mouse_rx.recv().await {
                                    let _ = window_cx.update(|window, cx| {
                                        mouse_view.update(cx, |view, cx| match event {
                                            HotkeyEvent::MouseMove(x, y, hwnd, title)
                                                if view.is_choosing =>
                                            {
                                                view.x_input.update(cx, |input, cx| {
                                                    input.set_value(&x.to_string(), window, cx);
                                                });
                                                view.y_input.update(cx, |input, cx| {
                                                    input.set_value(&y.to_string(), window, cx);
                                                });
                                                view.win_hwnd_input.update(cx, |input, cx| {
                                                    input.set_value(&hwnd, window, cx);
                                                });
                                                view.win_title_input.update(cx, |input, cx| {
                                                    input.set_value(&title, window, cx);
                                                });
                                                cx.notify();
                                            }
                                            HotkeyEvent::MouseReleased if view.is_choosing => {
                                                view.finish_selection(window, cx);
                                            }
                                            HotkeyEvent::AddMouse(x, y, hwnd, title)
                                                if view.watch_hotkey =>
                                            {
                                                view.add_hotkey_row(
                                                    window,
                                                    cx,
                                                    x,
                                                    y,
                                                    hwnd,
                                                    title,
                                                    OptCate::鼠标,
                                                );
                                            }
                                            HotkeyEvent::AddKeyboard(x, y, hwnd, title)
                                                if view.watch_hotkey =>
                                            {
                                                view.add_hotkey_row(
                                                    window,
                                                    cx,
                                                    x,
                                                    y,
                                                    hwnd,
                                                    title,
                                                    OptCate::键盘,
                                                );
                                            }
                                            HotkeyEvent::KeyboardInput(row_ix, value) => {
                                                view.data_table.update(cx, |table, cx| {
                                                    *table
                                                        .delegate()
                                                        .expected_keyboard_value
                                                        .lock()
                                                        .unwrap() = Some((row_ix, value.clone()));
                                                    if let Some(input) =
                                                        table.delegate().opt_type_inputs.get(row_ix)
                                                    {
                                                        input.update(cx, |input, cx| {
                                                            input.set_value(&value, window, cx);
                                                        });
                                                    }
                                                });
                                            }
                                            HotkeyEvent::ToggleExecution => {
                                                view.toggle_execution(cx);
                                            }
                                            HotkeyEvent::ExecutionFinished => {
                                                view.is_running = false;
                                                cx.notify();
                                            }
                                            _ => {}
                                        });
                                    });
                                }
                            })
                            .detach();
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
