#![windows_subsystem = "windows"]
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
use gpui_component::radio::{Radio, RadioGroup};
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_component::tooltip::Tooltip;
use gpui_component::*;
use gpui_component::{Icon, IconName};
use rdev::{Button as RdevButton, Event, EventType, Key as RdevKey, listen};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::{Duration, Instant};
use std::vec;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
    OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetActiveWindow, GetDoubleClickTime, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, MapVirtualKeyW, SendInput, VIRTUAL_KEY,
    VkKeyScanW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, GA_ROOT, GCL_STYLE, GetAncestor, GetClassLongPtrW, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetWindowTextW, IDC_CROSS, LoadCursorW, OCR_NORMAL,
    SM_CXDOUBLECLK, SM_CYDOUBLECLK, SPI_SETCURSORS, SPIF_SENDCHANGE, SW_HIDE, SW_SHOW,
    SendMessageW, SetCursorPos, SetForegroundWindow, SetSystemCursor, ShowWindow,
    SystemParametersInfoW, WHEEL_DELTA, WM_CHAR, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WindowFromPoint,
};

/// 鼠标消息 wParam 里的“哪个键正按下”标志（winuser.h 的 MK_*），
/// windows crate 只在 SystemServices 特性下导出，这里按消息格式自己写。
const MK_LBUTTON: usize = 0x0001;
const MK_RBUTTON: usize = 0x0002;
const MK_MBUTTON: usize = 0x0010;

/// 关于页里的下载地址。
const DOWNLOAD_URL: &str = "https://gitee.com/xarzhi/i-click-gpui/releases";

/// 关于页里的作者主页。
const AUTHOR_URL: &str = "https://www.52pojie.cn/home.php?mod=space&uid=2283786";

/// 关于页里的两张收款码：资源路径 + include_bytes! 进来的图。
const PAYMENT_IMAGES: [(&str, &[u8]); 2] = [
    ("payment/wechat.png", include_bytes!("assets/wechat.png")),
    ("payment/alipay.png", include_bytes!("assets/zhi-pay.png")),
];

/// 配置文件格式版本：以后结构变了可以按版本兼容。
/// 1 → 2：操作列表从一份变成多方案（v1 的老配置读进来就是「方案一」）。
const CONFIG_VERSION: u32 = 2;

/// 配置文件里存的一个方案。
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct SchemeConfig {
    #[serde(default)]
    name: String,
    #[serde(default)]
    rows: Vec<RowConfig>,
}

/// 保存/读取的配置文件内容：程序里的所有配置 + 每个方案的操作列表。
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ConfigFile {
    /// 必填：没有这个字段就说明不是本程序的配置文件。
    version: u32,
    #[serde(default)]
    saved_at: String,
    #[serde(default)]
    opt_one: OptOneConfig,
    #[serde(default)]
    opt_two: OptTwoConfig,
    #[serde(default)]
    hotkeys: HotkeyConfig,
    /// 所有方案（方案一…方案五）。
    #[serde(default)]
    schemes: Vec<SchemeConfig>,
    /// 保存时正处于哪个方案，读回来还切到那一个。
    #[serde(default)]
    active_scheme: usize,
    /// 当前方案的操作列表。老版本配置只有这一个字段，读的时候当成唯一的方案一。
    #[serde(default)]
    rows: Vec<RowConfig>,
}

/// 操作一页面上的配置。
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct OptOneConfig {
    always_on_top: bool,
    front_opt: bool,
    fuzzy_click: bool,
    /// 随机延迟：勾上以后新添的行，延迟在下面的范围里随机取
    random_delay: bool,
    random_delay_min: String,
    random_delay_max: String,
    fuzzy_round: String,
    loop_count: String,
    loop_gap: String,
    opt_gap: String,
    opt_cate: usize,
    hide_when_choose: bool,
    watch_hotkey: bool,
    x: String,
    y: String,
    win_hwnd: String,
    win_title: String,
}

impl Default for OptOneConfig {
    fn default() -> Self {
        Self {
            always_on_top: false,
            front_opt: false,
            fuzzy_click: false,
            random_delay: false,
            random_delay_min: "50".to_string(),
            random_delay_max: "200".to_string(),
            fuzzy_round: "0".to_string(),
            loop_count: "1".to_string(),
            loop_gap: "0".to_string(),
            opt_gap: "20".to_string(),
            opt_cate: 0,
            hide_when_choose: false,
            watch_hotkey: true,
            x: "0".to_string(),
            y: "0".to_string(),
            win_hwnd: String::new(),
            win_title: String::new(),
        }
    }
}

/// 操作二页面上的配置。
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct OptTwoConfig {
    click_gap: String,
    click_count: String,
}

impl Default for OptTwoConfig {
    fn default() -> Self {
        Self {
            click_gap: "20".to_string(),
            click_count: "0".to_string(),
        }
    }
}

/// 五个全局快捷键的绑定。
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct HotkeyConfig {
    record: String,
    toggle_run: String,
    mouse_event: String,
    keyboard_event: String,
    opt_two_run: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self::from(&HotkeySettings::default())
    }
}

impl From<&HotkeySettings> for HotkeyConfig {
    fn from(settings: &HotkeySettings) -> Self {
        Self {
            record: settings.record.clone(),
            toggle_run: settings.toggle_run.clone(),
            mouse_event: settings.mouse_event.clone(),
            keyboard_event: settings.keyboard_event.clone(),
            opt_two_run: settings.opt_two_run.clone(),
        }
    }
}

impl From<HotkeyConfig> for HotkeySettings {
    fn from(config: HotkeyConfig) -> Self {
        Self {
            record: config.record,
            toggle_run: config.toggle_run,
            mouse_event: config.mouse_event,
            keyboard_event: config.keyboard_event,
            opt_two_run: config.opt_two_run,
        }
    }
}

/// 表格里的一行。
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct RowConfig {
    x: usize,
    y: usize,
    #[serde(default)]
    win_hwnd: String,
    /// "鼠标" 或 "键盘"
    opt_cate: String,
    opt_type: String,
    #[serde(default)]
    win_title: Option<String>,
    #[serde(default)]
    delay: usize,
    #[serde(default = "default_execute_time")]
    execute_time: usize,
}

fn default_execute_time() -> usize {
    1
}

impl ConfigFile {
    /// 校验读进来的内容：版本对不对、方案个数、每一行的数据说不说得通。
    /// 只要有一处不对就报错，界面上原来的东西一点不动。
    fn validate(&self) -> Result<(), String> {
        if self.version == 0 || self.version > CONFIG_VERSION {
            return Err(format!(
                "不支持的配置文件版本 {}（本程序支持到 {}）",
                self.version, CONFIG_VERSION
            ));
        }
        if self.schemes.len() > MAX_SCHEMES {
            return Err(format!(
                "配置里有 {} 个方案，最多只支持 {MAX_SCHEMES} 个",
                self.schemes.len()
            ));
        }
        if self.schemes.is_empty() {
            validate_rows(&self.rows, "")?;
        } else {
            for (index, scheme) in self.schemes.iter().enumerate() {
                let name = if scheme.name.trim().is_empty() {
                    SCHEME_NAMES
                        .get(index)
                        .copied()
                        .unwrap_or("方案")
                        .to_string()
                } else {
                    scheme.name.clone()
                };
                validate_rows(&scheme.rows, &format!("「{name}」"))?;
            }
        }
        Ok(())
    }
}

/// 校验一批操作行；prefix 用来在报错里指出是哪个方案。
fn validate_rows(rows: &[RowConfig], prefix: &str) -> Result<(), String> {
    for (index, row) in rows.iter().enumerate() {
        let line = index + 1;
        match row.opt_cate.as_str() {
            "鼠标" => {
                if MouseOptType::from_label(&row.opt_type).is_none() {
                    return Err(format!(
                        "{prefix}第 {line} 行的操作方式「{}」不是有效的鼠标操作",
                        row.opt_type
                    ));
                }
            }
            "键盘" => {
                if row.opt_type.trim().is_empty() {
                    return Err(format!("{prefix}第 {line} 行是键盘操作，但没写按键"));
                }
            }
            other => {
                return Err(format!(
                    "{prefix}第 {line} 行的操作类型「{other}」只能是「鼠标」或「键盘」"
                ));
            }
        }
    }
    Ok(())
}

/// 在 [min, max] 里取一个随机延迟（含两端）；范围填反了就按 min 算。
fn random_delay_between(seed: &mut u64, min: usize, max: usize) -> usize {
    if max <= min {
        return min;
    }
    let span = (max - min + 1) as u64;
    min + (next_random(seed) % span) as usize
}

/// 本机当前日期，用来拼配置文件名：iclick_config_2025_09_18.json。
fn config_file_stem() -> String {
    let time = unsafe { GetLocalTime() };
    format!(
        "iclick_config_{:04}_{:02}_{:02}",
        time.wYear, time.wMonth, time.wDay
    )
}

/// 保存时间，写进配置文件里给人看。
fn local_datetime_text() -> String {
    let time = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
    )
}

/// 程序所在目录：配置文件就放这儿，便携版在哪都能找到。
fn app_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 保存配置的目标路径：程序目录下的 iclick_config_年月日.json。
fn config_save_path() -> PathBuf {
    app_dir().join(format!("{}.json", config_file_stem()))
}

/// 把相对路径按程序目录补全（拖到图标上的路径可能是相对的）。
fn resolve_config_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        app_dir().join(path)
    }
}

/// 打开/保存对话框要用到的宽字符串：跟着 OPENFILENAMEW 一起活到调用结束。
struct DialogText {
    filter: Vec<u16>,
    title: Vec<u16>,
    initial_dir: Vec<u16>,
}

impl DialogText {
    fn new(title: &str) -> Self {
        Self {
            filter: "配置文件 (*.json)\0*.json\0所有文件 (*.*)\0*.*\0\0"
                .encode_utf16()
                .collect(),
            title: title.encode_utf16().chain(std::iter::once(0)).collect(),
            initial_dir: app_dir()
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect(),
        }
    }
}

/// 对话框回填的宽字符串（以 0 结尾）转成路径。
fn wide_buffer_to_path(buffer: &[u16]) -> Option<PathBuf> {
    let length = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    if length == 0 {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
}

/// 弹系统「打开文件」对话框挑配置文件；返回 None 表示用户点了取消。
///
/// 注意：这个函数会跑一个模态消息循环，必须放在后台线程里调用（见 `spawn_file_dialog`），
/// 否则会和 gpui 自己的事件循环相互重入，直接把程序带崩。
fn pick_config_file(owner: HWND) -> Result<Option<PathBuf>, String> {
    let text = DialogText::new("选择要读取的配置文件");
    let mut buffer = vec![0u16; 1024];
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner,
        lpstrFilter: windows::core::PCWSTR(text.filter.as_ptr()),
        lpstrFile: windows::core::PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrInitialDir: windows::core::PCWSTR(text.initial_dir.as_ptr()),
        lpstrTitle: windows::core::PCWSTR(text.title.as_ptr()),
        Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    if !unsafe { GetOpenFileNameW(&mut options) }.as_bool() {
        return Ok(None);
    }
    Ok(wide_buffer_to_path(&buffer))
}

/// 弹系统「保存文件」对话框让用户挑保存位置，文件名默认是 iclick_config_年月日.json；
/// 返回 None 表示用户点了取消。同样必须在后台线程里调用。
fn pick_save_file(owner: HWND) -> Result<Option<PathBuf>, String> {
    let text = DialogText::new("选择配置保存位置");
    let default_path: Vec<u16> = config_save_path()
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let default_extension: Vec<u16> = "json".encode_utf16().chain(std::iter::once(0)).collect();
    let mut buffer = vec![0u16; 1024];
    let copy_len = default_path.len().min(buffer.len());
    buffer[..copy_len].copy_from_slice(&default_path[..copy_len]);

    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner,
        lpstrFilter: windows::core::PCWSTR(text.filter.as_ptr()),
        lpstrFile: windows::core::PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrInitialDir: windows::core::PCWSTR(text.initial_dir.as_ptr()),
        lpstrTitle: windows::core::PCWSTR(text.title.as_ptr()),
        lpstrDefExt: windows::core::PCWSTR(default_extension.as_ptr()),
        // OFN_OVERWRITEPROMPT：选到已存在的文件时先问一句要不要覆盖。
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    if !unsafe { GetSaveFileNameW(&mut options) }.as_bool() {
        return Ok(None);
    }
    Ok(wide_buffer_to_path(&buffer))
}

/// 「保存 / 读取」按钮按下的动作，交给后台线程去弹对话框。
#[derive(Clone, Copy)]
enum DialogKind {
    Save,
    Load,
}

/// 在后台线程里弹文件对话框，结果再发回界面线程套用。
///
/// 为什么不在界面线程直接弹：文件对话框自己会跑一个模态消息循环，界面线程被堵在里面时，
/// gpui 的事件循环就和它重入了 —— 一旦重入触发 panic，整程序直接没了（用户看到的就是
/// 「点一下保存程序就退出了」）。
fn spawn_file_dialog(kind: DialogKind, sender: async_channel::Sender<HotkeyEvent>) {
    // 模态对话框的属主：点按钮这会儿，本窗口就是当前活动窗口。
    // HWND 里是裸指针，不能跨线程传，转成整数带过去再转回来。
    let owner_value = unsafe { GetActiveWindow() }.0 as isize;
    std::thread::spawn(move || {
        let owner = HWND(owner_value as *mut std::ffi::c_void);
        // 文件对话框走 COM，后台线程得自己初始化（按 STA 来，跟资源管理器一致）。
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let picked = match kind {
            DialogKind::Save => pick_save_file(owner),
            DialogKind::Load => pick_config_file(owner),
        };
        if initialized {
            unsafe { CoUninitialize() };
        }
        let event = match kind {
            DialogKind::Save => HotkeyEvent::SavePicked(picked),
            DialogKind::Load => HotkeyEvent::LoadPicked(picked),
        };
        let _ = sender.try_send(event);
    });
}

/// 解析并校验一份配置文本（读取文件和拖进来的文件都走这里）。
fn parse_config(text: &str) -> Result<ConfigFile, String> {
    let config: ConfigFile =
        serde_json::from_str(text).map_err(|error| format!("不是本程序的配置文件：{error}"))?;
    config.validate()?;
    Ok(config)
}

/// 配置里的方案 → 界面用的方案列表。
/// 新配置带 schemes 就用它；老配置只有 rows，那就当成唯一的「方案一」。
fn schemes_of(config: &ConfigFile) -> Vec<Scheme> {
    let mut schemes: Vec<Scheme> = if config.schemes.is_empty() {
        vec![Scheme {
            name: SCHEME_NAMES[0].to_string(),
            rows: config.rows.iter().map(my_data_of).collect(),
        }]
    } else {
        config
            .schemes
            .iter()
            .enumerate()
            .map(|(index, scheme)| Scheme {
                name: if scheme.name.trim().is_empty() {
                    SCHEME_NAMES
                        .get(index)
                        .copied()
                        .unwrap_or("方案")
                        .to_string()
                } else {
                    scheme.name.clone()
                },
                rows: scheme.rows.iter().map(my_data_of).collect(),
            })
            .collect()
    };
    schemes.truncate(MAX_SCHEMES);
    schemes
}

/// 把配置里的一行还原成表格数据。
fn my_data_of(row: &RowConfig) -> MyData {
    MyData {
        x: row.x,
        y: row.y,
        win_hwnd: row.win_hwnd.clone(),
        opt_cate: if row.opt_cate == "键盘" {
            OptCate::键盘
        } else {
            OptCate::鼠标
        },
        opt_type: row.opt_type.clone(),
        win_title: row.win_title.clone(),
        delay: row.delay,
        execute_time: row.execute_time.max(1),
    }
}

/// 把一行表格数据转成配置里的样子。
fn row_config_of(row: &MyData) -> RowConfig {
    RowConfig {
        x: row.x,
        y: row.y,
        win_hwnd: row.win_hwnd.clone(),
        opt_cate: match row.opt_cate {
            OptCate::鼠标 => "鼠标".to_string(),
            OptCate::键盘 => "键盘".to_string(),
        },
        opt_type: row.opt_type.clone(),
        win_title: row.win_title.clone(),
        delay: row.delay,
        execute_time: row.execute_time,
    }
}

/// 把配置写成 json 文件。
fn write_config_file(config: &ConfigFile, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| format!("生成配置内容失败：{error}"))?;
    std::fs::write(path, json).map_err(|error| format!("写入 {} 失败：{error}", path.display()))
}

/// 把 panic 记到程序目录下的 iclick_panic.log。
///
/// 这是 GUI 程序（windows_subsystem = "windows"），没有控制台，panic 信息平时根本看不到，
/// 程序一退就什么线索都没了；写个日志文件，出事能查。
fn install_panic_logger() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|place| format!("{}:{}:{}", place.file(), place.line(), place.column()))
            .unwrap_or_else(|| "未知位置".to_string());
        let text = format!(
            "[{}] panic at {location}\n{info}\n{}\n\n",
            local_datetime_text(),
            std::backtrace::Backtrace::force_capture()
        );
        let path = app_dir().join("iclick_panic.log");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = file.write_all(text.as_bytes());
        }
    }));
}

/// 程序自己的资源：两张收款码走内嵌字节，其余（组件的图标等）交给 gpui-component 的资源包。
struct AppAssets {
    components: gpui_component_assets::Assets,
}

impl AppAssets {
    fn new() -> Self {
        Self {
            components: gpui_component_assets::Assets,
        }
    }
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = PAYMENT_IMAGES
            .iter()
            .find(|(resource, _)| *resource == path)
        {
            return Ok(Some(std::borrow::Cow::Borrowed(*bytes)));
        }
        self.components.load(path)
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        self.components.list(path)
    }
}

/// 这一次鼠标按下是不是落在输入框上。
///
/// 输入框外面那层容器会先收到 mouse down（事件从内层往外套），把这里置位；
/// 页面根节点的 handler 随后消费这个标记，决定要不要让正在编辑的输入框失焦。
static INPUT_PRESSED: AtomicBool = AtomicBool::new(false);

/// 挂在输入框外层容器上的 mouse down：标记「这一下点在输入框里」。
fn mark_input_press(_: &MouseDownEvent, _: &mut Window, _: &mut App) {
    INPUT_PRESSED.store(true, Ordering::Release);
}

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
    /// 操作二：开始/停止纯前台连点
    ToggleOptTwo,
    /// 操作二的连点已经点完（次数用完）
    OptTwoFinished,
    /// 开始/停止录制
    ToggleRecording,
    /// 修改快捷键的弹窗捕获到的新快捷键
    HotkeyCaptured(String),
    /// 读取一个配置文件（界面上点「读取」，或者把文件拖到程序图标上启动）
    LoadConfig(PathBuf),
    /// 保存对话框选好的路径：Ok(None) 是用户取消
    SavePicked(Result<Option<PathBuf>, String>),
    /// 读取对话框选好的路径：Ok(None) 是用户取消
    LoadPicked(Result<Option<PathBuf>, String>),
    /// 录制中捕捉到的一次操作。`delay` 是它距离上一个操作过了多少毫秒。
    RecordedInput {
        opt_cate: OptCate,
        opt_type: String,
        x: f64,
        y: f64,
        hwnd: String,
        title: String,
        delay: usize,
    },
    /// 录制中把刚记下的那一次点击并成了双击；`delay` 是第二次点击距上一次操作的间隔，
    /// 只有当并进上一行失败、需要补一行时才会用到。
    RecordedDoubleClick {
        opt_type: String,
        x: f64,
        y: f64,
        hwnd: String,
        title: String,
        delay: usize,
    },
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
    /// 四个全局快捷键的绑定，和监听线程共享。
    hotkeys: Arc<std::sync::Mutex<HotkeySettings>>,
    /// 弹窗是否正在等用户按下新快捷键（置位后监听线程只捕获按键）。
    hotkey_capture_flag: Arc<AtomicBool>,
    /// 正在修改快捷键的弹窗。
    hotkey_modal: Option<HotkeyModal>,
    /// 是否正在录制操作。
    is_recording: bool,
    record_flag: Arc<AtomicBool>,
    execution_flag: Arc<AtomicBool>,
    is_running: bool,
    /// 操作二的连点是否正在跑。
    opt_two_running: bool,
    opt_two_flag: Arc<AtomicBool>,
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
    /// 随机延迟：勾上以后新加进来的行，延迟在下面这个范围里随机取。
    random_delay: bool,
    random_delay_min_input: Entity<InputState>, // 随机延迟范围下限
    random_delay_max_input: Entity<InputState>, // 随机延迟范围上限
    /// 取随机延迟用的状态（xorshift）。
    delay_random_state: u64,
    loop_gap_ipt: Entity<InputState>, // 循环间隔
    opt_gap_ipt: Entity<InputState>,  // 操作间隔
    // 操作二（纯前台连点）的输入框；开始/停止快捷键用的是全局快捷键，没有输入框。
    click_gap_input: Entity<InputState>,   // 点击间隔
    click_count_input: Entity<InputState>, // 点击次数
    _input_subscriptions: Vec<Subscription>,
    data_table: Entity<TableState<MyTableDelegate>>,
    /// 读取/保存出错时的那行红字（显示在读取/保存按钮下面）；None 表示没出错。
    config_error: Option<SharedString>,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseOptType {
    左键单击,
    左键双击,
    滚轮上滑,
    滚轮下滑,
    滚轮单击,
    右键单击,
    右键双击,
    /// 只把光标移到这个点，不点（录制鼠标轨迹时用）
    鼠标移动,
}

/// “操作方式”下拉框里左键双击的文案；它同时也是行数据里保存的值，
/// 执行时按这个值判断该行要发一次单击还是双击。
const LEFT_DOUBLE_CLICK_LABEL: &str = "左键双击";

/// 鼠标移动这一项在行数据里的文案。
const MOUSE_MOVE_LABEL: &str = "鼠标移动";

static MOUSE_OPT_TYPE_LABELS: LazyLock<[SharedString; 8]> = LazyLock::new(|| {
    [
        "左键单击".into(),
        LEFT_DOUBLE_CLICK_LABEL.into(),
        "滚轮上滑".into(),
        "滚轮下滑".into(),
        "滚轮单击".into(),
        "右键单击".into(),
        "右键双击".into(),
        MOUSE_MOVE_LABEL.into(),
    ]
});

impl MouseOptType {
    /// 表格里的“操作方式”下拉框顺序，也是 [`MOUSE_OPT_TYPE_LABELS`] 的下标。
    const ALL: [MouseOptType; 8] = [
        Self::左键单击,
        Self::左键双击,
        Self::滚轮上滑,
        Self::滚轮下滑,
        Self::滚轮单击,
        Self::右键单击,
        Self::右键双击,
        Self::鼠标移动,
    ];

    fn index(&self) -> usize {
        match self {
            Self::左键单击 => 0,
            Self::左键双击 => 1,
            Self::滚轮上滑 => 2,
            Self::滚轮下滑 => 3,
            Self::滚轮单击 => 4,
            Self::右键单击 => 5,
            Self::右键双击 => 6,
            Self::鼠标移动 => 7,
        }
    }

    /// 下拉框里显示的文案，同时也是行数据里保存的值。
    fn label(&self) -> &'static SharedString {
        &MOUSE_OPT_TYPE_LABELS[self.index()]
    }

    /// 从行数据里保存的文案还原操作方式；空值或不认识的文案返回 None。
    fn from_label(label: &str) -> Option<Self> {
        let label = label.trim();
        Self::ALL
            .into_iter()
            .find(|kind| kind.label().as_ref() == label)
    }

    /// 双击需要在第一次点按之后再补一组按下/抬起。
    fn is_double_click(&self) -> bool {
        matches!(self, Self::左键双击 | Self::右键双击)
    }

    /// 双击对应的那一次单击；录制时用它确认上一行就是这次点击。
    fn single_click(&self) -> Option<Self> {
        match self {
            Self::左键双击 => Some(Self::左键单击),
            Self::右键双击 => Some(Self::右键单击),
            _ => None,
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

/// 可以自定义的全局快捷键。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HotkeyAction {
    /// 开始录制
    Record,
    /// 开始/暂停执行
    ToggleRun,
    /// 把鼠标当前位置记成一行鼠标操作
    MouseEvent,
    /// 把鼠标当前位置记成一行键盘操作
    KeyboardEvent,
    /// 操作二：开始/停止纯前台连点
    OptTwoRun,
}

impl HotkeyAction {
    const ALL: [HotkeyAction; 5] = [
        HotkeyAction::Record,
        HotkeyAction::ToggleRun,
        HotkeyAction::MouseEvent,
        HotkeyAction::KeyboardEvent,
        HotkeyAction::OptTwoRun,
    ];

    fn default_key(self) -> &'static str {
        match self {
            Self::Record => "F10",
            Self::ToggleRun => "F8",
            Self::MouseEvent => "F6",
            Self::KeyboardEvent => "F7",
            Self::OptTwoRun => "F9",
        }
    }

    /// 弹窗里显示的操作名。
    fn label(self) -> &'static str {
        match self {
            Self::Record => "开始录制",
            Self::ToggleRun => "开始/暂停",
            Self::MouseEvent => "鼠标事件",
            Self::KeyboardEvent => "键盘事件",
            Self::OptTwoRun => "操作二开始/停止",
        }
    }
}

/// 五个快捷键当前的绑定，键名和 [`keyboard_key_label`] 给出的标签一致（如 "F8"、"A"）。
///
/// 这份配置被监听线程共享：每次按键都现读现比，所以弹窗里改完（重新注册）
/// 下一次按键就按新绑定走，不需要另外向系统注销/注册热键。
#[derive(Clone, Debug)]
struct HotkeySettings {
    record: String,
    toggle_run: String,
    mouse_event: String,
    keyboard_event: String,
    opt_two_run: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            record: HotkeyAction::Record.default_key().to_string(),
            toggle_run: HotkeyAction::ToggleRun.default_key().to_string(),
            mouse_event: HotkeyAction::MouseEvent.default_key().to_string(),
            keyboard_event: HotkeyAction::KeyboardEvent.default_key().to_string(),
            opt_two_run: HotkeyAction::OptTwoRun.default_key().to_string(),
        }
    }
}

impl HotkeySettings {
    fn get(&self, action: HotkeyAction) -> &String {
        match action {
            HotkeyAction::Record => &self.record,
            HotkeyAction::ToggleRun => &self.toggle_run,
            HotkeyAction::MouseEvent => &self.mouse_event,
            HotkeyAction::KeyboardEvent => &self.keyboard_event,
            HotkeyAction::OptTwoRun => &self.opt_two_run,
        }
    }

    fn get_mut(&mut self, action: HotkeyAction) -> &mut String {
        match action {
            HotkeyAction::Record => &mut self.record,
            HotkeyAction::ToggleRun => &mut self.toggle_run,
            HotkeyAction::MouseEvent => &mut self.mouse_event,
            HotkeyAction::KeyboardEvent => &mut self.keyboard_event,
            HotkeyAction::OptTwoRun => &mut self.opt_two_run,
        }
    }

    fn set(&mut self, action: HotkeyAction, key: impl Into<String>) {
        *self.get_mut(action) = key.into();
    }

    /// 绑定值是否就是这次按下的键（忽略大小写，空绑定不算）。
    fn matches(bound: &str, pressed: &str) -> bool {
        let bound = bound.trim();
        !bound.is_empty() && bound.eq_ignore_ascii_case(pressed.trim())
    }

    /// 除了 `action`，还有哪个操作占用了 `key`。
    fn conflict(&self, action: HotkeyAction, key: &str) -> Option<HotkeyAction> {
        HotkeyAction::ALL
            .into_iter()
            .find(|candidate| *candidate != action && Self::matches(self.get(*candidate), key))
    }
}

/// 修改快捷键弹窗的状态。
#[derive(Clone, Debug)]
struct HotkeyModal {
    action: HotkeyAction,
    /// 待确认的新快捷键。
    draft: String,
    /// 是否正在等待用户按下新快捷键。
    capturing: bool,
    /// 与其它操作撞车时，撞上的那个操作。
    conflict: Option<HotkeyAction>,
}

fn lock_hotkeys(
    hotkeys: &Arc<std::sync::Mutex<HotkeySettings>>,
) -> std::sync::MutexGuard<'_, HotkeySettings> {
    hotkeys
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 监听线程里判断这次按下的键是不是某个操作当前的快捷键。
fn hotkey_pressed(
    hotkeys: &Arc<std::sync::Mutex<HotkeySettings>>,
    action: HotkeyAction,
    pressed: &str,
) -> bool {
    HotkeySettings::matches(lock_hotkeys(hotkeys).get(action), pressed)
}

/// 监听线程里跟踪的修饰键状态。快捷键和录制的组合键都要靠它拼键名。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModifierState {
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
}

impl ModifierState {
    /// 按下/松开一个键；返回 true 表示这个键本身就是修饰键。
    fn update(&mut self, key: &RdevKey, pressed: bool) -> bool {
        match key {
            RdevKey::ControlLeft | RdevKey::ControlRight => self.ctrl = pressed,
            RdevKey::ShiftLeft | RdevKey::ShiftRight => self.shift = pressed,
            RdevKey::Alt | RdevKey::AltGr => self.alt = pressed,
            RdevKey::MetaLeft | RdevKey::MetaRight => self.meta = pressed,
            _ => return false,
        }
        true
    }

    /// 组合键的文字前缀，顺序固定为 CTRL、SHIFT、ALT、WIN。
    fn prefixes(&self, shift_as_modifier: bool) -> Vec<&'static str> {
        let mut prefixes = Vec::new();
        if self.ctrl {
            prefixes.push("CTRL");
        }
        if self.shift && shift_as_modifier {
            prefixes.push("SHIFT");
        }
        if self.alt {
            prefixes.push("ALT");
        }
        if self.meta {
            prefixes.push("WIN");
        }
        prefixes
    }
}

/// 快捷键用的规范键名："F8"、"CTRL+SHIFT+A" 这样。
/// 修饰键顺序固定、字母统一大写，所以录制时写进去的绑定和按下时拼出来的键名一定一致；
/// 带上修饰键之后，Ctrl+F8 也不会误触发只绑了 F8 的操作。
fn hotkey_key_label(modifiers: &ModifierState, key: &RdevKey) -> String {
    let base = keyboard_key_label(key, false).to_uppercase();
    let mut parts = modifiers.prefixes(true);
    parts.push(&base);
    parts.join("+")
}

/// 只按“是哪个键”算出来的键名，不带修饰键。
/// 用来记「这个键已经被当成快捷键用掉了」，等它弹起时不再写进表格。
fn plain_key_label(key: &RdevKey) -> String {
    hotkey_key_label(&ModifierState::default(), key)
}

/// 表格里“键盘操作”保存的键名。普通打字保留原样的字符（大小写跟着 Shift/CapsLock 走），
/// 组合键写成 "CTRL+SHIFT+a"、"SHIFT+F5"，执行时按组合键发出去。
fn keyboard_row_label(modifiers: &ModifierState, caps_lock: bool, key: &RdevKey) -> String {
    // 带 Ctrl/Alt/Win 的组合键按“物理键 + 修饰键”记录：Shift 单独写出来，
    // 免得 CTRL+SHIFT+A 被记成 CTRL+A（那是另一个快捷键）。
    if modifiers.ctrl || modifiers.alt || modifiers.meta {
        let base = keyboard_key_label(key, false);
        let mut parts = modifiers.prefixes(true);
        parts.push(&base);
        return parts.join("+");
    }

    let base = keyboard_key_label(key, modifiers.shift ^ caps_lock);
    // 字母的 Shift 已经体现在大小写上（a / A），不用再写成 SHIFT+；
    // 数字、符号和功能键不行，得把 SHIFT 写出来，执行时才能打出 Shift+1 这种字符。
    let folded_into_case =
        base.chars().count() == 1 && base.chars().all(|ch| ch.is_ascii_alphabetic());
    let mut parts = modifiers.prefixes(!folded_into_case);
    parts.push(&base);
    parts.join("+")
}

/// 键盘操作的动作：录制的按下和弹起是两行，手填的键名默认按一整个按键（按下+弹起）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeyAction {
    /// 按下 + 弹起（不写前缀时的默认）
    Press,
    /// 只按下
    Down,
    /// 只弹起
    Up,
}

impl KeyAction {
    /// 表格里写在键名前面的前缀。
    fn prefix(self) -> &'static str {
        match self {
            Self::Press => "",
            Self::Down => "按下 ",
            Self::Up => "弹起 ",
        }
    }
}

/// 把表格里的键盘操作拆成（动作, 剩下的键名）。
/// "按下 a" → (Down, "a")；"弹起 CTRL" → (Up, "CTRL")；"a" → (Press, "a")。
fn split_key_action(label: &str) -> (KeyAction, &str) {
    let label = label.trim();
    if let Some(rest) = label.strip_prefix("按下") {
        return (KeyAction::Down, rest.trim());
    }
    if let Some(rest) = label.strip_prefix("弹起") {
        return (KeyAction::Up, rest.trim());
    }
    (KeyAction::Press, label)
}

/// 录制键盘事件时写进表格的文案：每个按下、弹起都各记一行，
/// 修饰键（CTRL/SHIFT/ALT）自己也会被记成普通行，所以这里只写物理键名。
fn recorded_key_label(action: KeyAction, caps_lock: bool, key: &RdevKey) -> String {
    format!("{}{}", action.prefix(), keyboard_key_label(key, caps_lock))
}

/// 把表格里的键盘操作拆成（修饰键虚拟键码, 主键名）。没有组合时修饰键是空的。
fn split_keyboard_combo(label: &str) -> (Vec<usize>, String) {
    let mut parts: Vec<&str> = label
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    let Some(base) = parts.pop() else {
        return (Vec::new(), String::new());
    };
    let mut modifiers = Vec::new();
    for part in parts {
        match part.to_uppercase().as_str() {
            "CTRL" | "CONTROL" => modifiers.push(0x11),
            "SHIFT" => modifiers.push(0x10),
            "ALT" => modifiers.push(0x12),
            "WIN" | "META" => modifiers.push(0x5B),
            _ => {}
        }
    }
    (modifiers, base.to_string())
}

/// 把按键标签转成 Kbd 能显示的 Keystroke。
/// gpui 用 “-” 分隔修饰键，所以 "CTRL+SHIFT+A" 要写成 "ctrl-shift-a"；
/// 给出的标签都解析得动，兜底只是为了不 panic。
fn hotkey_keystroke(label: &str) -> Keystroke {
    let source = label
        .split('+')
        .map(|part| part.trim().to_lowercase())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    Keystroke::parse(&source).unwrap_or_else(|_| Keystroke::parse("f1").expect("f1 一定是合法按键"))
}

/// 录制时把鼠标键映射成表格里的操作方式。
fn recorded_mouse_opt_type(button: RdevButton) -> Option<MouseOptType> {
    match button {
        RdevButton::Left => Some(MouseOptType::左键单击),
        RdevButton::Right => Some(MouseOptType::右键单击),
        RdevButton::Middle => Some(MouseOptType::滚轮单击),
        _ => None,
    }
}

/// 录制时把鼠标键映射成双击的操作方式；中键没有双击的说法。
fn recorded_double_opt_type(button: RdevButton) -> Option<MouseOptType> {
    match button {
        RdevButton::Left => Some(MouseOptType::左键双击),
        RdevButton::Right => Some(MouseOptType::右键双击),
        _ => None,
    }
}

/// 系统“双击范围”（像素），默认 4。
fn double_click_area() -> f64 {
    let x = unsafe { GetSystemMetrics(SM_CXDOUBLECLK) };
    let y = unsafe { GetSystemMetrics(SM_CYDOUBLECLK) };
    x.max(y).clamp(1, 32) as f64
}

/// 两次鼠标按下算不算一次双击：同一个键、间隔在系统双击时间之内、位置基本没动。
fn is_double_click(
    previous: Option<(RdevButton, Instant, f64, f64)>,
    button: RdevButton,
    now: Instant,
    x: f64,
    y: f64,
) -> bool {
    let Some((last_button, last_time, last_x, last_y)) = previous else {
        return false;
    };
    if last_button != button {
        return false;
    }
    if now.duration_since(last_time) > Duration::from_millis(double_click_time_ms()) {
        return false;
    }
    (x - last_x).abs() <= double_click_area() && (y - last_y).abs() <= double_click_area()
}

/// 新建行的默认操作方式：鼠标行是左键单击，键盘行留空等捕获按键。
fn default_opt_type(opt_cate: &OptCate) -> String {
    match opt_cate {
        OptCate::鼠标 => MouseOptType::左键单击.label().to_string(),
        OptCate::键盘 => String::new(),
    }
}

/// 界面上所有字体统一 12px；组件内部的字号是按 `Size` 反推的
/// （`text_size(size * 0.875)`），所以下拉框这种只认 Size 的控件要喂 12 / 0.875 进去。
/// 这样连它展开的菜单一起都是 12px，而高度、内边距仍与 `.small()` 一致。
fn component_size_for_12px() -> Pixels {
    px(12.0 / 0.875)
}

/// 模糊点击用的随机种子：每次开始执行取一个，够随机就行，不追求密码学强度。
fn random_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0);
    // 保证不为 0，xorshift 的种子全是 0 时会一直输出 0。
    nanos ^ std::process::id() as u64 | 1
}

/// xorshift64*：状态小、够快，模糊点击只要点位别老是同一个。
fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

/// 模糊点击：以 (x, y) 为中心，在 ±range 内各取一个随机点。
/// 比如范围 5、坐标 (100, 100)，取到的就是 x 95~105、y 95~105（不会取到负数）。
/// range 为 0 时原样返回，也就是不模糊。
fn fuzzy_point(x: usize, y: usize, range: usize, state: &mut u64) -> (usize, usize) {
    if range == 0 {
        return (x, y);
    }
    let span = range as u64 * 2 + 1;
    let offset_x = (next_random(state) % span) as i64 - range as i64;
    let offset_y = (next_random(state) % span) as i64 - range as i64;
    (
        (x as i64 + offset_x).max(0) as usize,
        (y as i64 + offset_y).max(0) as usize,
    )
}

/// 一轮操作走完之后要不要等“循环间隔”：只在还有下一轮的时候等，
/// 最后一轮结束后不用再空等一次（循环次数 0 表示无限循环，每轮之间都要等）。
fn waits_between_loops(loop_count: usize, completed_loops: usize) -> bool {
    loop_count == 0 || completed_loops < loop_count
}

/// 分片等待：中途被暂停（execution_flag 清零）就立刻返回 false，
/// 不会因为「循环间隔」填得很大而让暂停按钮像失灵一样。
/// 分片要短：暂停后最多再等一片就退出。
fn sleep_until_stopped(execution_flag: &Arc<AtomicBool>, duration: Duration) -> bool {
    const SLICE: Duration = Duration::from_millis(10);
    let mut slept = Duration::ZERO;
    while slept < duration {
        if !execution_flag.load(Ordering::Acquire) {
            return false;
        }
        let chunk = SLICE.min(duration - slept);
        thread::sleep(chunk);
        slept += chunk;
    }
    execution_flag.load(Ordering::Acquire)
}

/// 「操作间隔」的记账：每个操作开始前都要等这么久，
/// 但整场执行的第一个操作不用等（否则一按开始就白等一轮）。
struct OperationGap {
    /// 第一个操作是不是已经做过了。
    started: bool,
    gap_ms: usize,
}

impl OperationGap {
    fn new(gap_ms: usize) -> Self {
        Self {
            started: false,
            gap_ms,
        }
    }

    /// 取这一次操作开始前要等多少毫秒。
    fn wait_before_operation(&mut self) -> usize {
        if self.started {
            self.gap_ms
        } else {
            self.started = true;
            0
        }
    }
}

/// 操作二连点要不要继续：还按着快捷键、并且次数没用完。
/// 点击次数为 0 表示一直点，直到再按一次快捷键停下。
fn keeps_clicking(running: bool, click_count: usize, clicked: usize) -> bool {
    running && (click_count == 0 || clicked < click_count)
}

/// 操作二的每一次点击：在当前鼠标位置注入一次真实左键点击。
/// 用的是 SendInput，所以点到的就是光标下面那个窗口，光标本身不动。
fn click_at_cursor() {
    send_input_click(MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP);
}

/// 录制时算「这一步离上一步过了多久」，直接当成这一行的延迟。
/// 第一条没有上一步，延迟是 0（不然回放时会先干等一段开始前的发呆时间）。
fn take_recorded_delay(last: &mut Option<Instant>, now: Instant) -> usize {
    let delay = last.map_or(0, |previous| {
        now.duration_since(previous).as_millis() as usize
    });
    *last = Some(now);
    delay
}

/// 录制鼠标轨迹时的抽稀条件：位移够大、并且离上次记录够久才再记一行。
/// 鼠标移动事件一秒能来几百条，全记下来表格会被瞬间刷爆。
const MOVE_RECORD_MIN_DISTANCE: f64 = 4.0;
const MOVE_RECORD_MIN_INTERVAL: Duration = Duration::from_millis(25);

/// 这一次鼠标移动要不要记成一行。
fn should_record_move(last: Option<(Instant, f64, f64)>, now: Instant, x: f64, y: f64) -> bool {
    let Some((last_time, last_x, last_y)) = last else {
        return true;
    };
    if now.duration_since(last_time) < MOVE_RECORD_MIN_INTERVAL {
        return false;
    }
    (x - last_x).abs() >= MOVE_RECORD_MIN_DISTANCE || (y - last_y).abs() >= MOVE_RECORD_MIN_DISTANCE
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

/// 最多能有几个方案。
const MAX_SCHEMES: usize = 5;

/// 方案名按顺序取：方案一 … 方案五。
const SCHEME_NAMES: [&str; MAX_SCHEMES] = ["方案一", "方案二", "方案三", "方案四", "方案五"];

/// 新方案用哪个名字：取第一个还没被占用的（删掉中间某个方案后，
/// 再加回来应该补上那个空位，而不是撞名）。
fn first_free_scheme_name(used: &[&str]) -> &'static str {
    SCHEME_NAMES
        .iter()
        .find(|name| !used.contains(name))
        .copied()
        .unwrap_or(SCHEME_NAMES[0])
}

/// 一个方案：名字 + 它自己的一份操作列表（每个方案就是一个独立的表格）。
#[derive(Clone, Debug)]
struct Scheme {
    name: String,
    rows: Vec<MyData>,
}

struct MyTableDelegate {
    data: Vec<MyData>,
    /// 所有方案。第 active_scheme 个方案的内容就是上面的 data，
    /// 其余方案的表格内容先寄存在这里（切换方案时来回搬）。
    schemes: Vec<Scheme>,
    active_scheme: usize,
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
    opt_type_select_subscriptions: Vec<Subscription>,
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
        let opt_type_select_subscriptions: Vec<Subscription> = vec![];

        Self {
            data,
            schemes: vec![Scheme {
                name: SCHEME_NAMES[0].to_string(),
                rows: Vec::new(),
            }],
            active_scheme: 0,
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
            opt_type_select_subscriptions,
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
                  window,
                  cx: &mut Context<TableState<MyTableDelegate>>| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let Some(row_ix) = table
                    .delegate()
                    .opt_cate_selects
                    .iter()
                    .position(|candidate| candidate == select)
                else {
                    return;
                };
                let opt_cate = match value.as_ref() {
                    "鼠标" => OptCate::鼠标,
                    "键盘" => OptCate::键盘,
                    _ => return,
                };
                // 换了操作类型就换一套“操作方式”控件，行数据也要跟着复位成
                // 新控件的当前取值（鼠标默认左键单击，键盘留空等捕获按键），
                // 免得表格里显示的操作方式和实际执行的不是一回事。
                let opt_type_select = table.delegate().opt_type_selects.get(row_ix).cloned();
                let opt_type = default_opt_type(&opt_cate);
                {
                    let row = &mut table.delegate_mut().data[row_ix];
                    row.opt_cate = opt_cate;
                    row.opt_type = opt_type;
                }
                if let Some(opt_type_select) = opt_type_select {
                    opt_type_select.update(cx, |select, cx| {
                        select.set_selected_index(Some(IndexPath::new(0)), window, cx);
                    });
                }
                cx.notify();
            },
        );
        self.opt_cate_subscriptions.push(subscription);

        // 给新行创建“操作方式”下拉框 —— 用传进来的 window
        let options = SearchableVec::new(MouseOptType::ALL.to_vec());
        let opt_type_select: Entity<SelectState<SearchableVec<MouseOptType>>> = cx.new(|cx| {
            SelectState::new(
                options,
                // 恢复成行数据里已经保存的操作方式，而不是永远停在第一项。
                Some(IndexPath::new(
                    MouseOptType::from_label(&data.opt_type).map_or(0, |kind| kind.index()),
                )),
                window,
                cx,
            )
        });
        self.opt_type_selects.push(opt_type_select.clone());

        // 下拉框里换一种操作方式，就写回行数据，执行时按它决定发哪种鼠标输入。
        let subscription = cx.subscribe_in(
            &opt_type_select,
            window,
            move |table: &mut TableState<MyTableDelegate>,
                  select,
                  event: &SelectEvent<SearchableVec<MouseOptType>>,
                  _window,
                  cx: &mut Context<TableState<MyTableDelegate>>| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let Some(row_ix) = table
                    .delegate()
                    .opt_type_selects
                    .iter()
                    .position(|candidate| candidate == select)
                else {
                    return;
                };
                table.delegate_mut().data[row_ix].opt_type = value.to_string();
                cx.notify();
            },
        );
        self.opt_type_select_subscriptions.push(subscription);

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
            .text_size(px(12.0))
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
        menu.item(
            // 弹窗菜单的条目字号是组件内部写死的，用自己的元素才能跟着全局的 12px。
            PopupMenuItem::element(|_, _| div().text_size(px(12.0)).child("删除本行")).on_click(
                move |_, _, cx| {
                    table.update(cx, |table, cx| {
                        table.delegate_mut().remove_row(row_ix);
                        cx.notify();
                    });
                },
            ),
        )
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
                        .with_size(component_size_for_12px())
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
                            .with_size(component_size_for_12px())
                            .w(px(95.0)),
                    )
                    .into_any_element(),
                OptCate::键盘 => div()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .on_mouse_down(MouseButton::Left, mark_input_press)
                    .child(
                        Input::new(&self.opt_type_inputs[row_ix])
                            .size_full()
                            .w(px(95.0))
                            .px_1()
                            .text_size(px(12.0))
                            .rounded(px(4.0))
                            .py_0(),
                    )
                    .into_any_element(),
            },
            // 标题可能是空的（比如手工写的配置里没写），不能 unwrap。
            "win_title" => row.win_title.clone().unwrap_or_default().into_any_element(),
            "delay" => div()
                .flex()
                .items_center()
                .rounded_sm()
                .on_mouse_down(MouseButton::Left, mark_input_press)
                .child(
                    Input::new(&self.delay_inputs[row_ix])
                        .size_full()
                        .w(px(65.))
                        .px_1()
                        .text_size(px(12.0))
                        .rounded(px(4.))
                        .py_0(),
                )
                .into_any_element(),
            "execute_time" => div()
                .flex()
                .items_center()
                .rounded_sm()
                .on_mouse_down(MouseButton::Left, mark_input_press)
                .child(
                    Input::new(&self.execute_time_inputs[row_ix])
                        .size_full()
                        .w(px(70.))
                        .px_1()
                        .text_size(px(12.0))
                        .rounded(px(4.))
                        .py_0(),
                )
                .into_any_element(),
            _ => "".to_string().into_any_element(),
        };

        div()
            .p(px(0.0))
            .flex()
            .text_size(px(12.0))
            .items_center()
            .child(text)
    }
}

impl MyTableDelegate {
    /// 清空所有行（读取配置、切方案时用），跟着行一起建的输入框、下拉框和订阅也一起丢掉。
    fn clear_rows(&mut self) {
        self.data.clear();
        self.delay_inputs.clear();
        self.execute_time_inputs.clear();
        self.opt_type_inputs.clear();
        self.opt_cate_selects.clear();
        self.opt_type_selects.clear();
        self.delay_subscriptions.clear();
        self.execute_time_subscriptions.clear();
        self.opt_cate_subscriptions.clear();
        self.opt_type_subscriptions.clear();
        self.opt_type_select_subscriptions.clear();
        *self.focused_keyboard_row.lock().unwrap() = None;
        *self.expected_keyboard_value.lock().unwrap() = None;
    }

    /// 把当前正在编辑的表格内容存回它所属的方案。
    fn stash_active_scheme(&mut self) {
        if let Some(scheme) = self.schemes.get_mut(self.active_scheme) {
            scheme.rows = self.data.clone();
        }
    }

    /// 上面那排 tab 要显示的名字。
    fn scheme_names(&self) -> Vec<SharedString> {
        self.schemes
            .iter()
            .map(|scheme| SharedString::from(scheme.name.clone()))
            .collect()
    }

    /// 当前是第几个方案。
    fn active_scheme(&self) -> usize {
        self.active_scheme
    }

    /// 切换方案：先把当前方案的内容存回去，再把目标方案的行重新铺到表格里。
    fn switch_scheme(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        if index == self.active_scheme || index >= self.schemes.len() {
            return;
        }
        self.stash_active_scheme();
        self.active_scheme = index;
        let rows = self.schemes[index].rows.clone();
        self.clear_rows();
        for row in rows {
            self.add_row(window, cx, row);
        }
    }

    /// 新增一个方案并切过去；已经到上限就返回 false。
    fn add_scheme(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) -> bool {
        if self.schemes.len() >= MAX_SCHEMES {
            return false;
        }
        self.stash_active_scheme();
        let used: Vec<&str> = self
            .schemes
            .iter()
            .map(|scheme| scheme.name.as_str())
            .collect();
        let name = first_free_scheme_name(&used).to_string();
        self.schemes.push(Scheme {
            name,
            rows: Vec::new(),
        });
        self.active_scheme = self.schemes.len() - 1;
        // 新方案是空的，把表格清干净就是切过去了。
        self.clear_rows();
        let _ = (window, cx);
        true
    }

    /// 删掉第 index 个方案（方案一不给删）；删的是当前方案就切到前一个。
    /// 返回是否真的删了。
    fn remove_scheme(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> bool {
        if index == 0 || index >= self.schemes.len() {
            return false;
        }
        // 先把当前方案的内容存回去，免得待会儿搬数据时丢掉正在编辑的行。
        self.stash_active_scheme();
        self.schemes.remove(index);
        let target = if self.active_scheme == index {
            index - 1
        } else if self.active_scheme > index {
            self.active_scheme - 1
        } else {
            self.active_scheme
        };
        self.active_scheme = target;
        let rows = self.schemes[target].rows.clone();
        self.clear_rows();
        for row in rows {
            self.add_row(window, cx, row);
        }
        true
    }

    /// 清空当前方案的所有行；返回清掉的行数。
    fn clear_active_scheme(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> usize {
        let removed = self.data.len();
        self.clear_rows();
        if let Some(scheme) = self.schemes.get_mut(self.active_scheme) {
            scheme.rows.clear();
        }
        removed
    }

    /// 录制时把刚记下的那一次点击并成双击：只有最后一行正好是同一次单击（同样的键、
    /// 位置在系统双击范围之内）才成立，免得把别的操作误改成双击。
    /// 返回 true 表示这一行已经就地升级成双击。
    fn upgrade_last_mouse_row(&mut self, opt_type: MouseOptType, x: f64, y: f64) -> bool {
        let Some(single) = opt_type.single_click() else {
            return false;
        };
        let Some(row) = self.data.last_mut() else {
            return false;
        };
        if !matches!(row.opt_cate, OptCate::鼠标) {
            return false;
        }
        if row.opt_type.as_str() != single.label().as_ref() {
            return false;
        }
        let area = double_click_area();
        if (row.x as f64 - x).abs() > area || (row.y as f64 - y).abs() > area {
            return false;
        }
        row.opt_type = opt_type.label().to_string();
        true
    }

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
        let _ = self.opt_type_select_subscriptions.remove(row_ix);

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
        // 循环间隔：一轮表格走完之后，等这么久再开始下一轮。
        let loop_gap = self
            .loop_gap_ipt
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(0);
        // 操作间隔：每个操作做完之后，隔这么久才做下一个（跨行、跨轮都算）。
        let opt_gap = self
            .opt_gap_ipt
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(0);
        // 模糊点击：每次点击都在行坐标 ±模糊范围 内随机取点。
        let fuzzy_click = self.fuzzy_click;
        let fuzzy_range = if fuzzy_click {
            self.fuzzy_round_input
                .read(cx)
                .value()
                .parse::<usize>()
                .unwrap_or(0)
        } else {
            0
        };
        let front_opt = self.front_opt;

        self.is_running = true;
        self.execution_flag.store(true, Ordering::Release);
        let execution_flag = self.execution_flag.clone();
        let execution_event_tx = self.execution_event_tx.clone();
        thread::spawn(move || {
            let mut completed_loops = 0;
            // 每个执行线程自己一份随机种子，模糊点击用它取点位。
            let mut random_seed = random_seed();
            let mut operation_gap = OperationGap::new(opt_gap);
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

                        // 这一行自己的延迟：录制出来的值就是「距上一个操作过了多久」，
                        // 所以先等它，再等全局的操作间隔，然后才发这次操作。
                        // 这样回放的节奏和录制时一致，最后一行也不会多等一次。
                        if row.delay > 0
                            && !sleep_until_stopped(
                                &execution_flag,
                                Duration::from_millis(row.delay as u64),
                            )
                        {
                            break;
                        }

                        // 操作间隔：上一个操作和这一个之间的间隔。
                        let wait = operation_gap.wait_before_operation();
                        if wait > 0
                            && !sleep_until_stopped(
                                &execution_flag,
                                Duration::from_millis(wait as u64),
                            )
                        {
                            break;
                        }

                        // 一次点击取一个点：双击的两次点按共用它，否则就不是双击了。
                        let point = fuzzy_point(row.x, row.y, fuzzy_range, &mut random_seed);
                        send_row_input(hwnd, row, front_opt, point);
                    }
                }
                if !execution_flag.load(Ordering::Acquire) {
                    break;
                }
                completed_loops += 1;
                if waits_between_loops(loop_count, completed_loops)
                    && loop_gap > 0
                    && !sleep_until_stopped(&execution_flag, Duration::from_millis(loop_gap as u64))
                {
                    break;
                }
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

    /// 往表格里加一行。`delay` 就是这一行的「延迟(ms)」：
    /// 执行这一行之前先等这么久。录制出来的行填的是它距离上一步真实过去的时间，
    /// 热键插的行是 0。
    #[allow(clippy::too_many_arguments)]
    fn add_hotkey_row(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        x: f64,
        y: f64,
        hwnd: String,
        title: String,
        opt_cate: OptCate,
        opt_type: Option<String>,
        recorded_delay: usize,
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

        // 勾了「随机延迟」就不用录制到的那个间隔，改成在设定范围里随手取一个。
        let delay = if self.random_delay {
            let (min, max) = self.random_delay_range(cx);
            random_delay_between(&mut self.delay_random_state, min, max)
        } else {
            recorded_delay
        };

        // 录制出来的行带着当时用的操作方式，热键插的行用默认值。
        let opt_type = opt_type.unwrap_or_else(|| default_opt_type(&opt_cate));
        let data = MyData {
            x: x.round() as usize,
            y: y.round() as usize,
            win_hwnd: hwnd,
            opt_cate,
            opt_type,
            win_title: Some(title),
            delay,
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
        let opt_type = default_opt_type(&opt_cate);

        // 和快捷键插行一样：勾了「随机延迟」就在范围里随机取一个。
        let delay = if self.random_delay {
            let (min, max) = self.random_delay_range(cx);
            random_delay_between(&mut self.delay_random_state, min, max)
        } else {
            0
        };

        let data = MyData {
            x,
            y,
            win_hwnd,
            opt_cate,
            opt_type,
            win_title: Some(win_title),
            delay,
            execute_time: 1,
        };

        self.data_table.update(cx, |table, cx| {
            table.delegate_mut().add_row(window, cx, data);
        });
        cx.notify();
    }

    /// 某个操作当前绑定的快捷键。
    fn hotkey_label(&self, action: HotkeyAction) -> String {
        lock_hotkeys(&self.hotkeys).get(action).clone()
    }

    /// 界面上显示快捷键的小标签。
    fn hotkey_kbd(&self, action: HotkeyAction) -> Kbd {
        Kbd::new(hotkey_keystroke(&self.hotkey_label(action)))
            .outline()
            .text_size(px(12.0))
            .pt_1()
            .pb_1()
    }

    /// 打开“修改快捷键”弹窗，同时开始捕捉新按键。
    fn open_hotkey_modal(&mut self, action: HotkeyAction, cx: &mut Context<Self>) {
        self.hotkey_modal = Some(HotkeyModal {
            action,
            draft: self.hotkey_label(action),
            capturing: true,
            conflict: None,
        });
        self.start_hotkey_capture(cx);
    }

    /// 让监听线程把下一次按键交回来当新快捷键。
    fn start_hotkey_capture(&mut self, cx: &mut Context<Self>) {
        if let Some(modal) = self.hotkey_modal.as_mut() {
            modal.capturing = true;
            modal.conflict = None;
            self.hotkey_capture_flag.store(true, Ordering::Release);
            cx.notify();
        }
    }

    fn close_hotkey_modal(&mut self, cx: &mut Context<Self>) {
        self.hotkey_capture_flag.store(false, Ordering::Release);
        self.hotkey_modal = None;
        cx.notify();
    }

    /// 监听线程捕捉到的按键：先只放进草稿，等用户点“确定”。
    fn apply_captured_hotkey(&mut self, key: String, cx: &mut Context<Self>) {
        if let Some(modal) = self.hotkey_modal.as_mut() {
            modal.draft = key;
            modal.capturing = false;
            modal.conflict = None;
        }
        cx.notify();
    }

    /// 确认修改：写进共享的绑定表就算是“重新注册”，
    /// 监听线程下一次按键就会按新的键名比对。
    fn confirm_hotkey_modal(&mut self, cx: &mut Context<Self>) {
        let Some(modal) = self.hotkey_modal.clone() else {
            return;
        };
        let draft = modal.draft.trim().to_string();
        if draft.is_empty() {
            return;
        }

        if let Some(other) = lock_hotkeys(&self.hotkeys).conflict(modal.action, &draft) {
            if let Some(current) = self.hotkey_modal.as_mut() {
                current.capturing = false;
                current.conflict = Some(other);
            }
            cx.notify();
            return;
        }

        lock_hotkeys(&self.hotkeys).set(modal.action, draft);
        self.close_hotkey_modal(cx);
    }

    /// 开始/停止录制：录制期间监听线程把鼠标按键、滚轮和键盘按键都记成表格里的行。
    fn toggle_recording(&mut self, cx: &mut Context<Self>) {
        self.is_recording = !self.is_recording;
        self.record_flag.store(self.is_recording, Ordering::Release);
        cx.notify();
    }

    /// 记下出错信息（显示在读取/保存按钮下面那行红字），顺便刷新界面。
    /// 成功的提示不往界面上写，免得占地方。
    fn set_config_error(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.config_error = Some(message.into());
        cx.notify();
    }

    /// 清掉之前那条出错信息。
    fn clear_config_error(&mut self, cx: &mut Context<Self>) {
        if self.config_error.is_some() {
            self.config_error = None;
            cx.notify();
        }
    }

    /// 把界面上所有配置和每个方案的操作列表收集成一份配置。
    fn collect_config(&mut self, cx: &mut Context<Self>) -> ConfigFile {
        // 先把正在编辑的内容存回它所属的方案，免得漏掉刚敲进去的字。
        self.data_table
            .update(cx, |table, _cx| table.delegate_mut().stash_active_scheme());
        let text = |input: &Entity<InputState>| input.read(cx).value().to_string();
        let (schemes, active_scheme) = {
            let delegate = self.data_table.read(cx).delegate();
            (
                delegate
                    .schemes
                    .iter()
                    .map(|scheme| SchemeConfig {
                        name: scheme.name.clone(),
                        rows: scheme.rows.iter().map(row_config_of).collect(),
                    })
                    .collect::<Vec<_>>(),
                delegate.active_scheme,
            )
        };
        // rows 留一份当前方案的，老版本程序也能读这份配置。
        let rows = schemes
            .get(active_scheme)
            .map(|scheme| scheme.rows.clone())
            .unwrap_or_default();

        ConfigFile {
            version: CONFIG_VERSION,
            saved_at: local_datetime_text(),
            opt_one: OptOneConfig {
                always_on_top: self.always_on_top,
                front_opt: self.front_opt,
                fuzzy_click: self.fuzzy_click,
                random_delay: self.random_delay,
                random_delay_min: text(&self.random_delay_min_input),
                random_delay_max: text(&self.random_delay_max_input),
                fuzzy_round: text(&self.fuzzy_round_input),
                loop_count: text(&self.loop_count_input),
                loop_gap: text(&self.loop_gap_ipt),
                opt_gap: text(&self.opt_gap_ipt),
                opt_cate: self.opt_cate.unwrap_or(0),
                hide_when_choose: self.hide_when_choose,
                watch_hotkey: self.watch_hotkey,
                x: text(&self.x_input),
                y: text(&self.y_input),
                win_hwnd: text(&self.win_hwnd_input),
                win_title: text(&self.win_title_input),
            },
            opt_two: OptTwoConfig {
                click_gap: text(&self.click_gap_input),
                click_count: text(&self.click_count_input),
            },
            hotkeys: HotkeyConfig::from(&*lock_hotkeys(&self.hotkeys)),
            schemes,
            active_scheme,
            rows,
        }
    }

    /// 保存配置到用户选好的文件里。
    fn save_config_to(&mut self, cx: &mut Context<Self>, path: &Path) -> Result<PathBuf, String> {
        let config = self.collect_config(cx);
        write_config_file(&config, path)?;
        Ok(path.to_path_buf())
    }

    /// 读取配置文件并套用到界面上。
    fn load_config(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        path: &Path,
    ) -> Result<(), String> {
        let path = resolve_config_path(path);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("读取 {} 失败：{error}", path.display()))?;
        let config = parse_config(&text).map_err(|error| format!("{}：{error}", path.display()))?;
        self.apply_config(window, cx, config);
        Ok(())
    }

    /// 把配置套用到界面：勾选、下拉、输入框、快捷键，以及整张表格。
    fn apply_config(&mut self, window: &mut Window, cx: &mut Context<Self>, config: ConfigFile) {
        // 先按整份配置把方案算出来，下面再把各字段搬进界面。
        let schemes = schemes_of(&config);
        let active_scheme = config.active_scheme.min(schemes.len().saturating_sub(1));
        let opt_one = config.opt_one;
        self.always_on_top = opt_one.always_on_top;
        self.front_opt = opt_one.front_opt;
        self.fuzzy_click = opt_one.fuzzy_click;
        self.random_delay = opt_one.random_delay;
        self.opt_cate = Some(opt_one.opt_cate);
        self.hide_when_choose = opt_one.hide_when_choose;
        self.watch_hotkey = opt_one.watch_hotkey;
        self.hotkey_flag.store(self.watch_hotkey, Ordering::Release);

        let inputs: [(&Entity<InputState>, String); 12] = [
            (&self.fuzzy_round_input, opt_one.fuzzy_round),
            (&self.random_delay_min_input, opt_one.random_delay_min),
            (&self.random_delay_max_input, opt_one.random_delay_max),
            (&self.loop_count_input, opt_one.loop_count),
            (&self.loop_gap_ipt, opt_one.loop_gap),
            (&self.opt_gap_ipt, opt_one.opt_gap),
            (&self.x_input, opt_one.x),
            (&self.y_input, opt_one.y),
            (&self.win_hwnd_input, opt_one.win_hwnd),
            (&self.win_title_input, opt_one.win_title),
            (&self.click_gap_input, config.opt_two.click_gap),
            (&self.click_count_input, config.opt_two.click_count),
        ];
        for (input, value) in inputs {
            input.update(cx, |input, cx| input.set_value(&value, window, cx));
        }

        let hotkeys = HotkeySettings::from(config.hotkeys.clone());
        *lock_hotkeys(&self.hotkeys) = hotkeys;

        self.data_table.update(cx, |table, cx| {
            let delegate = table.delegate_mut();
            delegate.clear_rows();
            delegate.schemes = schemes;
            delegate.active_scheme = active_scheme;
            let rows = delegate.schemes[active_scheme].rows.clone();
            for row in rows {
                delegate.add_row(window, cx, row);
            }
        });
        cx.notify();
    }

    /// 上面那排方案 tab 被点了：切到对应方案。
    fn switch_scheme(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.data_table.update(cx, |table, cx| {
            table.delegate_mut().switch_scheme(index, window, cx)
        });
        cx.notify();
    }

    /// 「添加方案」：加一个空方案并切过去，最多 MAX_SCHEMES 个。
    /// 到上限时按钮本身会变灰（见渲染处），所以这里不用再提示什么。
    fn add_scheme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.data_table
            .update(cx, |table, cx| table.delegate_mut().add_scheme(window, cx));
        cx.notify();
    }

    /// 「随机延迟」用的范围（用户填的两个输入框，填反了会自动调正）。
    fn random_delay_range(&self, cx: &App) -> (usize, usize) {
        let min = self
            .random_delay_min_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(0);
        let max = self
            .random_delay_max_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(0);
        if max < min { (max, min) } else { (min, max) }
    }

    /// 删掉第 index 个方案；方案一不给删。
    fn remove_scheme(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.data_table.update(cx, |table, cx| {
            table.delegate_mut().remove_scheme(index, window, cx)
        });
        cx.notify();
    }

    /// 「清空列表」：把当前方案的操作全部清掉。
    fn clear_scheme_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.data_table.update(cx, |table, cx| {
            table.delegate_mut().clear_active_scheme(window, cx)
        });
        cx.notify();
    }

    /// 操作二的开始/停止：在当前鼠标位置连点，次数用完自己停，
    /// 点击次数填 0 就一直点，直到再按一次快捷键。
    fn toggle_opt_two(&mut self, cx: &mut Context<Self>) {
        if self.opt_two_running {
            self.opt_two_running = false;
            self.opt_two_flag.store(false, Ordering::Release);
            cx.notify();
            return;
        }

        let click_count = self
            .click_count_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(0);
        let interval = self
            .click_gap_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(0);

        self.opt_two_running = true;
        self.opt_two_flag.store(true, Ordering::Release);
        let running = self.opt_two_flag.clone();
        let event_tx = self.execution_event_tx.clone();
        thread::spawn(move || {
            let mut clicked = 0usize;
            while keeps_clicking(running.load(Ordering::Acquire), click_count, clicked) {
                click_at_cursor();
                clicked += 1;

                // 还有下一击才等点击间隔；次数用完就直接收工。
                if !keeps_clicking(running.load(Ordering::Acquire), click_count, clicked) {
                    break;
                }
                if interval > 0
                    && !sleep_until_stopped(&running, Duration::from_millis(interval as u64))
                {
                    break;
                }
            }
            running.store(false, Ordering::Release);
            let _ = event_tx.try_send(HotkeyEvent::OptTwoFinished);
        });
        cx.notify();
    }

    /// 修改快捷键的弹窗：铺满窗口的遮罩 + 居中的小面板。
    /// 遮罩自己不响应点击（只挡住下面的表格），关掉弹窗要点“取消”。
    fn render_hotkey_modal(&mut self, modal: &HotkeyModal, cx: &mut Context<Self>) -> AnyElement {
        let mut panel = div()
            .w(px(300.0))
            .p(px(14.0))
            .rounded(px(6.0))
            .border(px(1.0))
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(12.0))
                    .mb(px(10.0))
                    .child(format!("修改快捷键：{}", modal.action.label())),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p(px(10.0))
                    .rounded(px(4.0))
                    .border(px(1.0))
                    .border_color(if modal.capturing {
                        cx.theme().primary
                    } else {
                        cx.theme().border
                    })
                    .child(
                        Kbd::new(hotkey_keystroke(&modal.draft))
                            .outline()
                            .text_size(px(12.0)),
                    )
                    .child(
                        div()
                            .ml(px(8.0))
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(if modal.capturing {
                                "请按下新的快捷键，可配合 Ctrl/Shift/Alt"
                            } else {
                                "已捕捉到新快捷键"
                            }),
                    ),
            );

        if let Some(other) = modal.conflict {
            panel = panel.child(
                div()
                    .mt(px(8.0))
                    .text_size(px(12.0))
                    .text_color(cx.theme().danger)
                    .child(format!(
                        "“{}”已经是“{}”的快捷键了，请换一个键",
                        modal.draft,
                        other.label()
                    )),
            );
        }

        div()
            .id("hotkey_modal_backdrop")
            .occlude() // 遮罩要把鼠标事件全挡住，别让点空白处点到下面的表格
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(cx.theme().overlay)
            .flex()
            .items_center()
            .justify_center()
            .child(
                panel.child(
                    div()
                        .flex()
                        .justify_end()
                        .mt(px(14.0))
                        .child(
                            Button::new("hotkey_recapture")
                                .label("重新录制")
                                .small()
                                .text_size(px(12.0))
                                .mr(px(8.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.start_hotkey_capture(cx);
                                })),
                        )
                        .child(
                            Button::new("hotkey_cancel")
                                .label("取消")
                                .small()
                                .text_size(px(12.0))
                                .mr(px(8.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_hotkey_modal(cx);
                                })),
                        )
                        .child(
                            Button::new("hotkey_confirm")
                                .label("确定")
                                .small()
                                .text_size(px(12.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.confirm_hotkey_modal(cx);
                                })),
                        ),
                ),
            )
            .into_any_element()
    }

    /// 关于页：整页水平居中，自上而下是标题、下载地址、收款码，然后是功能、快捷键和用法。
    /// 内容比窗口高，所以整页可以滚动。
    fn render_about(&mut self, cx: &mut Context<Self>) -> AnyElement {
        // 每段文字都给一个确定的宽度、并显式打开换行：
        // gpui 只在 white_space 是 Normal 且拿得到确定宽度时，才会按宽度换行来测量高度，
        // 否则高度按「一行」算，换行后的第二行就会压到下一段上、滚动条也滚不到底。
        let text_width = px(470.0);
        let line = move |text: &str| {
            div()
                .w(text_width)
                .flex_shrink_0()
                .whitespace_normal()
                .text_center()
                .mb(px(6.0))
                .child(text.to_string())
        };
        let heading = move |text: &str| {
            div()
                .w(text_width)
                .flex_shrink_0()
                .whitespace_normal()
                .text_center()
                .font_weight(FontWeight::SEMIBOLD)
                .mt(px(16.0))
                .mb(px(8.0))
                .child(text.to_string())
        };

        let features = [
            "操作一：把每一步操作列成表格，逐行编辑坐标、操作类型、操作方式、窗口、延迟和执行次数。",
            "操作方式共 8 种：左键/右键/中键的单击与双击、滚轮上滑下滑、鼠标移动、键盘按下与弹起。",
            "两种执行方式：后台把消息直接投给目标窗口（不抢鼠标、可后台点击），或前台用 SendInput 发真实点击。",
            "录制（默认 F10）：鼠标移动、点击、滚轮、键盘按下与弹起，全部按真实间隔记进表格的「延迟」列。",
            "模糊点击：每次点击都在坐标 ±模糊范围 内随机取点，避免老是点在同一个像素上。",
            "循环控制：循环次数（0 表示无限次）、循环间隔（两轮之间等多久）、操作间隔（两步之间等多久）。",
            "操作二：纯前台连点，按下快捷键后在鼠标当前位置连点，可以指定次数，也可以一直点到再按一次快捷键。",
            "其它开关：窗口置顶、选择坐标时隐藏本窗口、监听热键、前台/后台操作。",
        ];
        let hotkeys = [
            "F8：开始 / 暂停执行（按下立即停止）",
            "F9：操作二 开始 / 停止连点",
            "F10：开始 / 停止录制",
            "F6：在鼠标位置插入一行鼠标操作",
            "F7：在鼠标位置插入一行键盘操作",
        ];
        let steps = [
            "1. 用「获取坐标方式一」的十字按钮，或者方式二的快捷键，把要用到的坐标加进表格。",
            "2. 在表格里调整每一行的操作方式、延迟和执行次数。",
            "3. 按需要设置模糊点击、循环次数、循环间隔、操作间隔。",
            "4. 点「开始」或按 F8 执行；再按一次 F8 立即停止。",
            "5. 嫌麻烦就直接按 F10 录一遍，录出来的表格还能继续手工微调。",
        ];

        let mut column = div()
            .flex()
            .flex_col()
            .items_center()
            .w_full()
            .p(px(8.0))
            .child(
                div()
                    // 全界面统一 12px，标题只用加粗区分层级。
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .flex_shrink_0()
                    .whitespace_normal()
                    .child("iClick-gpui"),
            )
            .child(
                div()
                    .w(text_width)
                    .text_center()
                    .whitespace_normal()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .mb(px(14.0))
                    .child("一个用 Rust + GPUI 写的 Windows 连点 / 宏录制工具"),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .mb(px(16.0))
                    .child(div().mr(px(4.0)).child("下载地址："))
                    .child(
                        div()
                            .id("download_link")
                            .cursor(CursorStyle::PointingHand)
                            .text_color(rgb(0x1f44fa))
                            .text_decoration_1()
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.open_url(DOWNLOAD_URL);
                            }))
                            .child(DOWNLOAD_URL),
                    ),
            )
            .child(
                div()
                    .w(text_width)
                    .flex_shrink_0()
                    .whitespace_normal()
                    .text_center()
                    .mb(px(8.0))
                    .child("如果您觉得本软件对您有帮助，可以把钱交给我保管"),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .justify_center()
                    .gap(px(12.0))
                    .mb(px(6.0))
                    .children(PAYMENT_IMAGES.iter().map(|(resource, _)| {
                        img(*resource)
                            .w(px(168.0))
                            .rounded(px(6.0))
                            .into_any_element()
                    })),
            );

        column = column.child(heading("功能介绍"));
        for feature in features {
            column = column.child(line(feature));
        }

        column = column.child(heading("快捷键（在界面上点键位标签就能改）"));
        for hotkey in hotkeys {
            column = column.child(line(hotkey));
        }

        column = column.child(heading("使用步骤"));
        for step in steps {
            column = column.child(line(step));
        }

        column = column.child(heading("其它"));
        column = column.child(line("技术栈：Rust + GPUI + gpui-component"));
        // 作者那一行里，「吾爱破解 xarzhi」是可以点的超链接。
        column = column.child(
            div()
                .flex()
                .justify_center()
                .items_center()
                .mb(px(6.0))
                .child("作者：")
                .child(
                    div()
                        .id("author_link")
                        .cursor(CursorStyle::PointingHand)
                        .text_color(rgb(0x1f44fa))
                        .text_decoration_1()
                        .on_click(cx.listener(|_, _, _, cx| {
                            cx.open_url(AUTHOR_URL);
                        }))
                        .child("吾爱破解 xarzhi"),
                ),
        );

        // 滚动容器里只放这一列：滚动范围按这一列的真实高度算，底部的文字才滚得出来。
        // 末尾多留一段空白：换行段落的高度累计起来会差几个像素，留白了最后一行才露得出来。
        div()
            .id("about_scroll")
            // 同上：占满剩余高度（滚动容器要有确定高度才滚得对）。
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .child(column.child(div().h(px(64.0))))
            .into_any_element()
    }

    /// 操作二：纯前台连点。开始/停止快捷键和操作一那几个一样，
    /// 点小标签就能开弹窗改绑定。
    fn render_opt_two(&mut self, cx: &mut Context<Self>) -> Div {
        div()
            // 同上：占满 tab 栏以下的剩余空间，别用 size_full。
            .flex_1()
            .min_h(px(0.0))
            .bg(rgb(0xf6f6f6))
            .p(px(8.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(px(16.0))
                    .child("纯前台连点，按下快捷键开始点击，鼠标在哪就点"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .mb(px(14.0))
                    .child(div().child("开始/停止快捷键："))
                    .child(
                        div()
                            .ml(px(8.0))
                            .id("hotkey_opt_two")
                            .cursor(CursorStyle::PointingHand)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_hotkey_modal(HotkeyAction::OptTwoRun, cx);
                            }))
                            .child(self.hotkey_kbd(HotkeyAction::OptTwoRun)),
                    )
                    .child(div().ml(px(10.0)).text_color(rgb(0x1f44fa)).child(
                        if self.opt_two_running {
                            "连点中，再按一次停止"
                        } else {
                            ""
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .mb(px(14.0))
                    .child(div().child("点击间隔："))
                    .child(
                        div()
                            .ml(px(8.0))
                            .w(px(46.0))
                            .on_mouse_down(MouseButton::Left, mark_input_press)
                            .child(
                                Input::new(&self.click_gap_input)
                                    .small()
                                    .text_size(px(12.0)),
                            ),
                    )
                    .child(div().ml(px(6.0)).child("ms")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(div().child("点击次数："))
                    .child(
                        div()
                            .ml(px(8.0))
                            .w(px(46.0))
                            .on_mouse_down(MouseButton::Left, mark_input_press)
                            .child(
                                Input::new(&self.click_count_input)
                                    .small()
                                    .text_size(px(12.0)),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(6.0))
                            .text_size(px(12.0))
                            .text_color(cx.theme().muted_foreground)
                            .child("0 表示一直点"),
                    ),
            )
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

/// 按键名对应的虚拟键码：先查常用表，再让系统按当前键盘布局翻译单字符（标点等）。
fn keyboard_virtual_key_for(value: &str) -> Option<usize> {
    if let Some(key) = keyboard_virtual_key(value) {
        return Some(key);
    }
    let mut chars = value.trim().chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let scanned = unsafe { VkKeyScanW(ch as u16) };
    // 返回值的低字节是虚拟键码，-1 表示这个字符在当前布局上打不出来。
    (scanned != -1).then(|| scanned as usize & 0xff)
}

fn key_input(vk: usize, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk as u16),
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                ..Default::default()
            },
        },
    }
}

/// 需要的话把目标窗口切到前台。已经在最前面就不要再调 SetForegroundWindow：
/// 这个调用在窗口不愿让出焦点时可能阻塞很久，而连点是一击一次，卡一次就慢一拍。
fn ensure_foreground(hwnd: HWND) {
    unsafe {
        let top_level = GetAncestor(hwnd, GA_ROOT);
        let target = if top_level.0.is_null() {
            hwnd
        } else {
            top_level
        };
        if GetForegroundWindow() != target {
            let _ = SetForegroundWindow(target);
        }
    }
}

fn send_input_key(hwnd: HWND, key: &str) {
    ensure_foreground(hwnd);

    let (action, key) = split_key_action(key);
    let (modifiers, base) = split_keyboard_combo(key);
    // 单个字符且不带修饰键：直接注入字符，中文、标点都发得出去。
    let is_character = modifiers.is_empty() && base.chars().count() == 1;
    let inputs = if !is_character {
        let Some(vk) = keyboard_virtual_key_for(&base) else {
            return;
        };
        // 组合键：修饰键按下 → 主键按下（抬起）→ 修饰键反序抬起。
        // 「只按下」不补抬起，「只弹起」不补按下，这样录制出来的按住过程能还原。
        let mut inputs: Vec<INPUT> = Vec::new();
        if action != KeyAction::Up {
            inputs.extend(modifiers.iter().map(|vk| key_input(*vk, false)));
            inputs.push(key_input(vk, false));
        }
        if action != KeyAction::Down {
            inputs.push(key_input(vk, true));
            inputs.extend(modifiers.iter().rev().map(|vk| key_input(*vk, true)));
        }
        inputs
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

/// 该行的操作方式是否为左键双击（只有测试直接用它断言双击语义）。
#[cfg(test)]
fn is_left_double_click(opt_type: &str) -> bool {
    MouseOptType::from_label(opt_type) == Some(MouseOptType::左键双击)
}

/// 系统“双击速度”对应的判定时间（毫秒），默认 500。
fn double_click_time_ms() -> u64 {
    (unsafe { GetDoubleClickTime() } as u64).clamp(200, 900)
}

/// 一次双击中两次点按之间的间隔。
/// 必须大于 0：间隔为 0 时，按时间差判定双击的程序（部分游戏、自绘控件）
/// 会把两下并成一下；又必须远小于系统双击判定时间，否则就不是双击而是两次单击。
fn double_click_gap() -> Duration {
    Duration::from_millis((double_click_time_ms() / 8).clamp(15, 60))
}

fn mouse_input(flags: MOUSE_EVENT_FLAGS, mouse_data: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                mouseData: mouse_data,
                ..Default::default()
            },
        },
    }
}

/// 注入一组完整的按下/抬起。
fn send_input_click(down: MOUSE_EVENT_FLAGS, up: MOUSE_EVENT_FLAGS) {
    let click = [mouse_input(down, 0), mouse_input(up, 0)];
    unsafe {
        let _ = SendInput(&click, std::mem::size_of::<INPUT>() as i32);
    }
}

/// 注入一次滚轮；滚动量放在 mouseData 里，向下滚是负值。
fn send_input_wheel(delta: i32) {
    let wheel = [mouse_input(MOUSEEVENTF_WHEEL, delta as u32)];
    unsafe {
        let _ = SendInput(&wheel, std::mem::size_of::<INPUT>() as i32);
    }
}

/// 前台操作：把光标移到目标点，再用 SendInput 注入真实鼠标输入。
/// 双击就注入两组“按下/抬起”：SendInput 走的是系统输入队列，
/// 第二组会被系统自身的双击判定识别，并自动合成 WM_*BUTTONDBLCLK，
/// 所以这里不需要（也不应该）自己补发那条消息。
fn send_input_mouse(hwnd: HWND, row: &MyData, point: (usize, usize)) {
    ensure_foreground(hwnd);
    unsafe {
        let mut cursor = POINT {
            x: point.0 as i32,
            y: point.1 as i32,
        };
        let _ = windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut cursor);
        let _ = SetCursorPos(cursor.x, cursor.y);
    }

    // 认不出来的操作方式按左键单击处理，保证老数据也点得动。
    let opt_type = MouseOptType::from_label(&row.opt_type).unwrap_or(MouseOptType::左键单击);
    let (down, up) = match opt_type {
        MouseOptType::左键单击 | MouseOptType::左键双击 => {
            (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP)
        }
        MouseOptType::右键单击 | MouseOptType::右键双击 => {
            (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP)
        }
        MouseOptType::滚轮单击 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        MouseOptType::滚轮上滑 => {
            send_input_wheel(WHEEL_DELTA as i32);
            return;
        }
        MouseOptType::滚轮下滑 => {
            send_input_wheel(-(WHEEL_DELTA as i32));
            return;
        }
        // 鼠标移动就是把光标挪过去（上面已经 SetCursorPos），不点。
        MouseOptType::鼠标移动 => return,
    };

    // 一次单击 = 按下 + 抬起，一次 SendInput 注入一组完整点按。
    send_input_click(down, up);
    if opt_type.is_double_click() {
        thread::sleep(double_click_gap());
        send_input_click(down, up);
    }
}

/// 窗口类是否带 CS_DBLCLKS：只有带这个样式的窗口类才会收到 WM_*BUTTONDBLCLK。
fn class_accepts_double_click(hwnd: HWND) -> bool {
    let class_style = unsafe { GetClassLongPtrW(hwnd, GCL_STYLE) } as u32;
    class_style & CS_DBLCLKS.0 != 0
}

/// 后台操作：直接把鼠标消息投递给目标窗口。
/// SendMessage 绕过了系统输入队列，系统不会替我们判定双击，
/// 因此双击的消息序列要按窗口类样式自己补全：
///   带 CS_DBLCLKS 的类：DOWN → UP → DBLCLK → UP（第二次按下用 WM_*BUTTONDBLCLK 代替）
///   不带 CS_DBLCLKS 的类：真实双击对它就是两次普通按下，永远不会出现 DBLCLK 消息
fn send_message_mouse(hwnd: HWND, row: &MyData, point: (usize, usize)) {
    // 认不出来的操作方式按左键单击处理，保证老数据也点得动。
    let opt_type = MouseOptType::from_label(&row.opt_type).unwrap_or(MouseOptType::左键单击);

    // 滚轮消息的位置参数是屏幕坐标，和按键消息的客户区坐标不一样，单独发。
    match opt_type {
        MouseOptType::滚轮上滑 => {
            send_message_wheel(hwnd, point, WHEEL_DELTA as i32);
            return;
        }
        MouseOptType::滚轮下滑 => {
            send_message_wheel(hwnd, point, -(WHEEL_DELTA as i32));
            return;
        }
        _ => {}
    }

    let x = (point.0 as u32) & 0xffff;
    let y = (point.1 as u32) & 0xffff;
    let lparam = LPARAM(((y << 16) | x) as isize);

    // 鼠标移动：只给目标窗口发一条 WM_MOUSEMOVE。
    if opt_type == MouseOptType::鼠标移动 {
        unsafe {
            let _ = SendMessageW(hwnd, WM_MOUSEMOVE, WPARAM(0), lparam);
        }
        return;
    }

    // 三种按键各自的按下/抬起/双击消息，以及按下时 wParam 里的按钮标志。
    let (down_message, up_message, double_message, button_flag) = match opt_type {
        MouseOptType::左键单击 | MouseOptType::左键双击 => {
            (WM_LBUTTONDOWN, WM_LBUTTONUP, WM_LBUTTONDBLCLK, MK_LBUTTON)
        }
        MouseOptType::右键单击 | MouseOptType::右键双击 => {
            (WM_RBUTTONDOWN, WM_RBUTTONUP, WM_RBUTTONDBLCLK, MK_RBUTTON)
        }
        MouseOptType::滚轮单击 => (WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MBUTTONDBLCLK, MK_MBUTTON),
        MouseOptType::滚轮上滑 | MouseOptType::滚轮下滑 => unreachable!("滚轮上面已经返回"),
        MouseOptType::鼠标移动 => unreachable!("鼠标移动上面已经返回"),
    };

    unsafe {
        let _ = SendMessageW(hwnd, down_message, WPARAM(button_flag), lparam);
        let _ = SendMessageW(hwnd, up_message, WPARAM(0), lparam);

        if !opt_type.is_double_click() {
            return;
        }

        // 两次点按之间留出节奏，让按时间差自行判定双击的程序也能识别。
        thread::sleep(double_click_gap());
        let second_down = if class_accepts_double_click(hwnd) {
            double_message
        } else {
            down_message
        };
        let _ = SendMessageW(hwnd, second_down, WPARAM(button_flag), lparam);
        let _ = SendMessageW(hwnd, up_message, WPARAM(0), lparam);
    }
}

/// 后台发一次滚轮：WM_MOUSEWHEEL 的位置参数用屏幕坐标，
/// 而滚动量放在 wParam 的高 16 位（向下滚是负值）。
fn send_message_wheel(hwnd: HWND, point: (usize, usize), delta: i32) {
    let mut screen = POINT {
        x: point.0 as i32,
        y: point.1 as i32,
    };
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut screen);
    }
    let x = (screen.x as u32) & 0xffff;
    let y = (screen.y as u32) & 0xffff;
    let lparam = LPARAM(((y << 16) | x) as isize);
    let wparam = ((delta as u32) & 0xffff) << 16;
    unsafe {
        let _ = SendMessageW(hwnd, WM_MOUSEWHEEL, WPARAM(wparam as usize), lparam);
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

fn send_row_input(hwnd: HWND, row: &MyData, front_opt: bool, point: (usize, usize)) {
    unsafe {
        match row.opt_cate {
            OptCate::鼠标 => {
                if front_opt {
                    send_input_mouse(hwnd, row, point);
                } else {
                    send_message_mouse(hwnd, row, point);
                }
            }
            OptCate::键盘 => {
                if front_opt {
                    send_input_key(hwnd, &row.opt_type);
                    return;
                }

                let (action, key_text) = split_key_action(&row.opt_type);
                let (modifiers, base) = split_keyboard_combo(key_text);
                // 单个字符、不带修饰键、而且是完整一按：老规矩，直接发字符消息。
                let plain_character =
                    action == KeyAction::Press && modifiers.is_empty() && base.chars().count() == 1;
                if plain_character {
                    for unit in base.encode_utf16() {
                        let _ = SendMessageW(hwnd, WM_CHAR, WPARAM(unit as usize), LPARAM(0));
                    }
                    return;
                }
                let Some(key) = keyboard_virtual_key_for(&base) else {
                    for unit in base.encode_utf16() {
                        let _ = SendMessageW(hwnd, WM_CHAR, WPARAM(unit as usize), LPARAM(0));
                    }
                    return;
                };
                // 组合键：修饰键按下 → 主键按下（抬起）→ 修饰键反序抬起；
                // 「只按下」/「只弹起」按录制的语义只发一半。
                let wparam = WPARAM(key);
                if action != KeyAction::Up {
                    for modifier in &modifiers {
                        let _ = SendMessageW(
                            hwnd,
                            WM_KEYDOWN,
                            WPARAM(*modifier),
                            keyboard_message_lparam(*modifier, false),
                        );
                    }
                    let _ = SendMessageW(
                        hwnd,
                        WM_KEYDOWN,
                        wparam,
                        keyboard_message_lparam(key, false),
                    );
                    if key == 0x0D {
                        let _ = SendMessageW(hwnd, WM_CHAR, WPARAM(0x0D), LPARAM(0));
                    }
                }
                if action != KeyAction::Down {
                    let _ =
                        SendMessageW(hwnd, WM_KEYUP, wparam, keyboard_message_lparam(key, true));
                    for modifier in modifiers.iter().rev() {
                        let _ = SendMessageW(
                            hwnd,
                            WM_KEYUP,
                            WPARAM(*modifier),
                            keyboard_message_lparam(*modifier, true),
                        );
                    }
                }
            }
        }
    }
}

impl Render for IClick {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 弹窗单独构建：它要盖在整页上面，所以放在最后一个子元素。
        let hotkey_modal = self.hotkey_modal.clone();
        let hotkey_modal_layer = hotkey_modal
            .map(|modal| self.render_hotkey_modal(&modal, cx))
            .into_iter();

        div()
            .relative()
            .size_full()
            // 必须是 flex 容器：上面是 tab 栏，下面各页面用 flex_1 占满剩余高度。
            // 少了这一行就是块级布局，子元素的 flex_1 完全不生效，
            // 页面会按内容高度排版，表格区域直接塌成 0 高。
            .flex()
            .flex_col()
            .text_size(px(12.0))
            // 点在输入框以外的地方就让正在编辑的输入框失焦（点在输入框里时不抢）。
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, window, cx| {
                if INPUT_PRESSED.swap(false, Ordering::AcqRel) {
                    return;
                }
                window.blur();
                cx.notify();
            }))
            .child(
                TabBar::new("tabs")
                    .small()
                    .text_size(px(12.0))
                    .selected_index(self.active_tab)
                    .segmented()
                    .on_click(cx.listener(|view, index, _, cx| {
                        view.active_tab = *index;
                        cx.notify();
                    }))
                    .children(self.tabs.iter().map(|tab_name| {
                        Tab::new()
                            .label(tab_name.clone())
                            .text_size(px(12.0))
                            .cursor(CursorStyle::PointingHand)
                    })),
            )
            .child(if self.active_tab == 0 {
                div()
                    .p(px(5.0))
                    // 竖排：上面两块固定高度，表格占满剩下的空间。
                    // 注意用 flex_1 而不是 size_full：根节点里上面还有一排 tab，
                    // 用 size_full 会按整窗高度算，整页就比可用区域高出一个 tab 栏，
                    // 表格底部会跑到窗口外面去。min_h(0) 保证内部不再按内容撑开。
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .mb(px(10.0))
                            .flex_shrink_0()
                            // 高度写死：这一行是 auto 高时子元素不会被拉伸，
                            // 左列和右边大框就会各自按内容高度排，底边对不齐。
                            // 250 = 左边两个框各 120 + 中间间距 10（右边大框的布局保持不变）。
                            .h(px(250.0))
                            .child(
                                div()
                                    .w(px(200.0))
                                    .mr(px(10.0))
                                    // 和右边大框一样撑满这一行的高度：这样第二个框
                                    // （flex_1）会自动补足，两块底边就齐平了。
                                    .h_full()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        GroupBox::new()
                                            .title("获取坐标方式一")
                                            .title_style(StyleRefinement::default().text_size(px(12.0)))
                                            .gap(px(4.0))
                                            .mb(px(10.0))
                                            .flex_1()
                                            .child(
                                                div()
                                                    .w_full()
                                                    .text_size(px(12.0))
                                                    .flex_1()
                                                    .h(px(100.0))
                                                    .border(px(1.0))
                                                    .border_color(cx.theme().border)
                                                    .p(px(5.))
                                                    .flex()
                                                    .flex_col()
                                                    .justify_between()
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
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
                                                            .text_size(px(12.0))
                                                            // Radio 自己会把字号设成 text_base，
                                                            // 所以每个选项都要单独压回 12px。
                                                            .children([
                                                                Radio::new("mouse-event")
                                                                    .label("鼠标事件")
                                                                    .text_size(px(12.0)),
                                                                Radio::new("keyboard-event")
                                                                    .label("键盘事件")
                                                                    .text_size(px(12.0)),
                                                            ])
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
                                                                .text_size(px(12.0))
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
                                            .flex_1()
                                            .title_style(
                                                StyleRefinement::default()
                                                    .text_size(px(12.0))
                                            )
                                            .child(
                                                div()
                                                    .w_full()
                                                    .flex_1()
                                            		.h_full()
                                                    .border(px(1.0))
                                                    .border_color(cx.theme().border)
                                                    .flex()
                                                    .p_1()
                                                    .flex_col()
                                                    .justify_between()
                                                    .h(px(95.))
                                                    .child(
                                                        Checkbox::new("front_check")
                                                            .mb(px(5.))
                                                            .label("监听热键")
                                                            .text_size(px(12.0))
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
                                                            .mb(px(5.))
                                                            .child(
                                                                div()
                                                                    .child("鼠标事件快捷键："),
                                                            )
                                                            .child(
                                                                div()
                                                                    .id("hotkey_mouse_event")
                                                                    .flex_1()
                                                                    .cursor(
                                                                        CursorStyle::PointingHand,
                                                                    )
                                                                    .on_click(cx.listener(
                                                                        |this, _, _, cx| {
                                                                            this.open_hotkey_modal(
                                                                                HotkeyAction::MouseEvent,
                                                                                cx,
                                                                            );
                                                                        },
                                                                    ))
                                                                    .child(
                                                                        self.hotkey_kbd(
                                                                            HotkeyAction::MouseEvent,
                                                                        )
                                                                        .flex_1(),
                                                                    ),
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
                                                                div()
                                                                    .id("hotkey_keyboard_event")
                                                                    .flex_1()
                                                                    .cursor(
                                                                        CursorStyle::PointingHand,
                                                                    )
                                                                    .on_click(cx.listener(
                                                                        |this, _, _, cx| {
                                                                            this.open_hotkey_modal(
                                                                                HotkeyAction::KeyboardEvent,
                                                                                cx,
                                                                            );
                                                                        },
                                                                    ))
                                                                    .child(
                                                                        self.hotkey_kbd(
                                                                            HotkeyAction::KeyboardEvent,
                                                                        )
                                                                        .flex_1(),
                                                                    ),
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
                                    // 高度不写死：和左边那一列一起被拉伸到同一高度，
                                    // 两边的底边自然就齐平了。
                                    .flex()
                                    .justify_center()
                                    .p(px(5.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .flex()
                                            .flex_col()
                                            // 行距统一 8px，内容从上往下排：
                                            // 之前用 justify_between 把剩余空间平摊，而每行本身高度不同
                                            // （带按钮的行更高），看起来就疏密不均、下面的行还被压得很低。
                                            .gap(px(8.0))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
                                                    .h(px(26.0))

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
															div().text_size(px(12.0)).mr(px(10.)).child("窗口置顶：")
														).child(
														   Checkbox::new("always_on_top")
                                                            .label("")
                                                            .text_size(px(12.0))
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
                                                            .text_size(px(12.0))
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
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
                                                    .h(px(26.0))
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
															div().mr(px(10.)).text_size(px(12.0)).child("前台操作：")
														)
														.child( Checkbox::new("front_check")
                                                            .label("")
                                                            .text_size(px(12.0))
                                                            .checked(self.front_opt)

													)
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .justify_between()
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
                                                    .h(px(26.0))
                                                            .w(px(140.0))
                                                            .child(
                                                                Button::new("read_btn")
                                                                    .label("读取")
                                                                    .small()
                                                                    .text_size(px(12.0))
                                                                    .mr(px(10.0))
                                                                    .cursor(
                                                                        CursorStyle::PointingHand,
                                                                    )
                                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                                        // 对话框放到后台线程去弹，避免堵住 gpui 的事件循环。
                                                                        spawn_file_dialog(
                                                                            DialogKind::Load,
                                                                            this.execution_event_tx.clone(),
                                                                        );
                                                                        cx.notify();
                                                                    }))
                                                            )
                                                            .child(
                                                                Button::new("save_btn")
                                                                    .label("保存")
                                                                    .small()
                                                                    .text_size(px(12.0))
                                                                    .cursor(
                                                                        CursorStyle::PointingHand,
                                                                    )
                                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                                        // 同样是后台线程弹保存对话框，用户选完再回传路径。
                                                                        spawn_file_dialog(
                                                                            DialogKind::Save,
                                                                            this.execution_event_tx.clone(),
                                                                        );
                                                                        cx.notify();
                                                                    })),
                                                            ),
                                                    ),
                                            )
                                            // 只有真出错时才加这一行（None 时完全不占位，
                                            // 否则它会白占一行高度，把「循环次数」以下的整块往下挤）。
                                            .children(self.config_error.clone().map(|error| {
                                                div()
                                                    .text_size(px(12.0))
                                                    .flex_shrink_0()
                                                    .whitespace_normal()
                                                    .text_color(rgb(0xd03030))
                                                    .child(error)
                                                    .into_any_element()
                                            }))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
                                                    .h(px(26.0))
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
																			Tooltip::new("表格中所有操作的总执行次数，0代表无限次")
																				.text_size(px(12.0))
																				.build(window, cx)
																	}),
                                                            )
                                                            .child(div().w(px(80.0)).on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                                Input::new(&self.loop_count_input).small().text_size(px(12.0)),
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
                                                                    .child(
                                                                        if self.is_recording {
                                                                            "录制中"
                                                                        } else {
                                                                            "开始录制"
                                                                        },
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .w(px(80.0))
                                                                    .id("hotkey_record")
                                                                    .cursor(
                                                                        CursorStyle::PointingHand,
                                                                    )
                                                                    .on_click(cx.listener(
                                                                        |this, _, _, cx| {
                                                                            this.open_hotkey_modal(
                                                                                HotkeyAction::Record,
                                                                                cx,
                                                                            );
                                                                        },
                                                                    ))
                                                                    .child(self.hotkey_kbd(
                                                                        HotkeyAction::Record,
                                                                    )),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
                                                    .h(px(26.0))
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
															div().mr(px(10.)).text_size(px(12.0)).child("模糊点击：")
														).child(  Checkbox::new("fuzzy_click")
                                                            .label("")
                                                            .text_size(px(12.0))
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
                                                                div().w(px(80.0)).on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                                    Input::new(
                                                                        &self.fuzzy_round_input,
                                                                    ).small()
                                                                    .text_size(px(12.0))
                                                                    .suffix(div().child("px")),
                                                                ),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                // 随机延迟 + 随机延迟范围，接在模糊点击/模糊范围下面。
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
                                                    .h(px(26.0))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .cursor(CursorStyle::PointingHand)
                                                            .id("random_delay")
                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                this.random_delay = !this.random_delay;
                                                                cx.notify();
                                                            }))
                                                            .child(
                                                                div()
                                                                    .mr(px(10.))
                                                                    .text_size(px(12.0))
                                                                    .child("随机延迟："),
                                                            )
                                                            .child(
                                                                Checkbox::new("random_delay_box")
                                                                    .label("")
                                                                    .text_size(px(12.0))
                                                                    .checked(self.random_delay),
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
                                                                    .child("随机延迟范围"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .w(px(48.0))
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        mark_input_press,
                                                                    )
                                                                    .child(
                                                                        Input::new(
                                                                            &self.random_delay_min_input,
                                                                        )
                                                                        .small()
                                                                        .text_size(px(12.0)),
                                                                    ),
                                                            )
                                                            .child(div().mx(px(4.0)).child("~"))
                                                            .child(
                                                                div()
                                                                    .w(px(48.0))
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        mark_input_press,
                                                                    )
                                                                    .child(
                                                                        Input::new(
                                                                            &self.random_delay_max_input,
                                                                        )
                                                                        .small()
                                                                        .text_size(px(12.0)),
                                                                    ),
                                                            )
                                                            .child(div().ml(px(6.0)).child("ms")),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    // 每行统一 26px 高：行内容有高有低（带按钮的更高），
                                                    // 不固定高度的话行距看着还是不均匀。
                                                    .h(px(26.0))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .child(
                                                                div()
                                                                    .mr(px(10.0))
                                                                    .child("操作间隔："),
                                                            )
                                                            .child(div().w(px(80.0)).on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                                Input::new(&self.opt_gap_ipt).small().text_size(px(12.0)).suffix(div().child("ms"))
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
                                                                div().w(px(80.0)).on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                                    Input::new(
                                                                        &self.loop_gap_ipt,
                                                                    ).small()
                                                                    .text_size(px(12.0))
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
                                                        div()
                                                            .id("hotkey_toggle_run")
                                                            .cursor(
                                                                CursorStyle::PointingHand,
                                                            )
                                                            .on_click(cx.listener(
                                                                |this, _, _, cx| {
                                                                    this.open_hotkey_modal(
                                                                        HotkeyAction::ToggleRun,
                                                                        cx,
                                                                    );
                                                                },
                                                            ))
                                                            .child(
                                                                self.hotkey_kbd(
                                                                    HotkeyAction::ToggleRun,
                                                                )
                                                                .min_w(px(100.)),
                                                            ),
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
							.child(div().on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                Input::new(&self.x_input).small().text_size(px(12.0)).w(px(55.)),
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
							.child(div().on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                Input::new(&self.y_input).small().text_size(px(12.0)).w(px(55.)),
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
							.child(div().flex_1().on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                Input::new(&self.win_hwnd_input).small().text_size(px(12.0)),
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
							.child(div().flex_1().on_mouse_down(MouseButton::Left, mark_input_press).child(
                                                Input::new(&self.win_title_input).small().text_size(px(12.0)),
							))
						)
					)
                    .child({
                        // 方案 tab（左） + 添加方案 / 清空列表（右），然后下面才是表格。
                        let scheme_names = self.data_table.read(cx).delegate().scheme_names();
                        let active_scheme = self.data_table.read(cx).delegate().active_scheme();
                        let scheme_count = scheme_names.len();
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .flex_shrink_0()
                            // .mt(px(8.0))
                            .mb(px(6.0))
                            .child(
                                TabBar::new("scheme_tabs")
                                    .small()
                                    .text_size(px(12.0))
                                    .selected_index(active_scheme)
                                    .segmented()
                                    .on_click(cx.listener(|this, index, window, cx| {
                                        this.switch_scheme(*index, window, cx);
                                    }))
                                    .children(scheme_names.into_iter().enumerate().map(
                                        |(index, name)| {
                                            // 标签文字用一个显式 12px 的 div 包着：
                                            // Tab 内部会给标签套上自己的字号，外面无论怎么设
                                            // text_size 都会被盖掉，只有写在这层才生效。
                                            let tab = Tab::new()
                                                .child(div().text_size(px(12.0)).child(name))
                                                .cursor(CursorStyle::PointingHand);
                                            // 方案一不给删，其余每个后面挂一个 ✕。
                                            if index == 0 {
                                                tab
                                            } else {
                                                tab.suffix(
                                                    div()
                                                        .id(("del_scheme", index))
                                                        .cursor(CursorStyle::PointingHand)
                                                        .text_size(px(12.0))
                                                        .text_color(cx.theme().muted_foreground)
                                                        .pr_1()
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                // 点 ✕ 只删方案，不要顺带切成这个方案。
                                                                cx.stop_propagation();
                                                                this.remove_scheme(index, window, cx);
                                                            },
                                                        ))
                                                        .child("✕"),
                                                )
                                            }
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .child(
                                        Button::new("add_scheme_btn")
                                            .label("添加方案")
                                            .small()
                                            .text_size(px(12.0))
                                            .mr(px(8.0))
                                            .cursor(CursorStyle::PointingHand)
                                            // 到 5 个就变灰，不用再弹文字提示。
                                            .disabled(scheme_count >= MAX_SCHEMES)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.add_scheme(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("clear_rows_btn")
                                            .label("清空列表")
                                            .small()
                                            .text_size(px(12.0))
                                            .cursor(CursorStyle::PointingHand)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.clear_scheme_rows(window, cx);
                                            })),
                                    ),
                            )
                    })
                    .child(
                        // 表格占满剩下的空间（flex_1 给出确定高度，DataTable 内部自己滚动）。
                        // min_h(0) 是必须的：否则它会按内容高度（行数多时）把整页顶出去。
                        // 底部留 5px，不贴着窗口边。
                        div()
                            .w_full()
                            .flex_1()
                            .min_h(px(0.0))
                            .mb(px(5.0))
                            .child(DataTable::new(&self.data_table).with_size(px(20.))),
                    )
                    .into_any_element()
            } else if self.active_tab == 1 {
                self.render_opt_two(cx).into_any_element()
            } else if self.active_tab == 2 {
                self.render_about(cx)
            } else {
                div().into_any_element()
            })
            .children(hotkey_modal_layer)
    }
}
use regex::Regex;
fn main() {
    install_panic_logger();
    let app = gpui_platform::application().with_assets(AppAssets::new());

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
                            title: Some("iClick-gpui v1.0.0 By：吾爱破解：xarzhi".into()),
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
                        // 随机延迟范围的两个输入框（毫秒）
                        let random_delay_min_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("50")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let random_delay_max_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("200")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        // 操作二（纯前台连点）的输入框；开始/停止快捷键走全局快捷键
                        let click_gap_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("20")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let click_count_input = cx.new(|cx| {
                            InputState::new(window, cx)
                                .default_value("0")
                                .pattern(Regex::new(r"^[0-9]\d*$").unwrap())
                        });
                        let input_subscriptions = vec![
                            subscribe_select_all(&loop_count_input, cx),
                            subscribe_select_all(&fuzzy_round_input, cx),
                            subscribe_select_all(&opt_gap_ipt, cx),
                            subscribe_select_all(&loop_gap_ipt, cx),
                            subscribe_select_all(&x_input, cx),
                            subscribe_select_all(&y_input, cx),
                            subscribe_select_all(&win_hwnd_input, cx),
                            subscribe_select_all(&win_title_input, cx),
                            subscribe_select_all(&random_delay_min_input, cx),
                            subscribe_select_all(&random_delay_max_input, cx),
                            subscribe_select_all(&click_gap_input, cx),
                            subscribe_select_all(&click_count_input, cx),
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
                        let hotkey_flag = Arc::new(AtomicBool::new(true));
                        let execution_flag = Arc::new(AtomicBool::new(false));
                        let record_flag = Arc::new(AtomicBool::new(false));
                        let opt_two_flag = Arc::new(AtomicBool::new(false));
                        let hotkeys = Arc::new(std::sync::Mutex::new(HotkeySettings::default()));
                        let hotkey_capture_flag = Arc::new(AtomicBool::new(false));
                        let (mouse_tx, mouse_rx) = async_channel::unbounded::<HotkeyEvent>();
                        // 把配置文件直接拖到程序图标上时，路径会作为第一个参数传进来。
                        let startup_config = std::env::args_os().nth(1).map(PathBuf::from);
                        let startup_config_tx = mouse_tx.clone();

                        let view = cx.new(|_| IClick {
                            active_tab: 0,
                            tabs: vec![
                                "操作一".to_string(),
                                "操作二".to_string(),
                                "关于".to_string(),
                            ],
                            loop_count_input,
                            fuzzy_round_input,
                            random_delay: false,
                            random_delay_min_input,
                            random_delay_max_input,
                            delay_random_state: random_seed(),
                            loop_gap_ipt,
                            opt_gap_ipt,
                            _input_subscriptions: input_subscriptions,
                            data_table: state,
                            config_error: None,
                            always_on_top: false,
                            front_opt: false,
                            fuzzy_click: false,
                            opt_cate: Some(0),
                            hide_when_choose: false,
                            watch_hotkey: true,
                            hotkey_flag: hotkey_flag.clone(),
                            hotkeys: hotkeys.clone(),
                            hotkey_capture_flag: hotkey_capture_flag.clone(),
                            hotkey_modal: None,
                            is_recording: false,
                            record_flag: record_flag.clone(),
                            execution_flag: execution_flag.clone(),
                            is_running: false,
                            opt_two_running: false,
                            opt_two_flag: opt_two_flag.clone(),
                            execution_event_tx: mouse_tx.clone(),
                            is_choosing: false,
                            choosing_flag: choosing_flag.clone(),
                            hidden_window: Arc::new(std::sync::Mutex::new(None)),
                            x_input,
                            y_input,
                            win_hwnd_input,
                            win_title_input,
                            click_gap_input,
                            click_count_input,
                        });
                        let listener_flag = choosing_flag.clone();
                        let hotkey_listener_flag = hotkey_flag.clone();
                        let listener_hotkeys = hotkeys.clone();
                        let listener_capture_flag = hotkey_capture_flag.clone();
                        let listener_record_flag = record_flag.clone();
                        let listener_execution_flag = execution_flag.clone();
                        let listener_opt_two_flag = opt_two_flag.clone();
                        let keyboard_input_row = focused_keyboard_row.clone();
                        std::thread::spawn(move || {
                            let mut modifiers = ModifierState::default();
                            let mut caps_lock = false;
                            // 录制时用来把两次快速点击并成一次双击。
                            let mut last_click: Option<(RdevButton, Instant, f64, f64)> = None;
                            // 录制时用来给鼠标轨迹抽稀：位置没怎么动就不记。
                            let mut last_move: Option<(Instant, f64, f64)> = None;
                            // 已经当成快捷键用掉的按键（键名不带修饰键），
                            // 它们弹起时不再往表格里写。
                            let mut hotkey_keys_down: Vec<String> = Vec::new();
                            // 上一个被记下的操作发生的时刻，用来算每行的「延迟」。
                            let mut last_recorded_at: Option<Instant> = None;
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
                                } else if let EventType::MouseMove { x, y } = event.event_type {
                                    // 录制模式：把鼠标轨迹也记成行。
                                    // 鼠标移动事件一秒能来几百条，所以按「距离 + 间隔」抽稀，
                                    // 不然表格会被瞬间刷爆。
                                    if listener_record_flag.load(Ordering::Acquire) {
                                        let now = Instant::now();
                                        if should_record_move(last_move, now, x, y) {
                                            let (client_x, client_y, hwnd, title) = cursor_info();
                                            last_move = Some((now, x, y));
                                            let delay =
                                                take_recorded_delay(&mut last_recorded_at, now);
                                            let _ = mouse_tx.try_send(HotkeyEvent::RecordedInput {
                                                opt_cate: OptCate::鼠标,
                                                opt_type: MOUSE_MOVE_LABEL.to_string(),
                                                x: client_x,
                                                y: client_y,
                                                hwnd,
                                                title,
                                                delay,
                                            });
                                        }
                                    }
                                } else if let EventType::ButtonPress(button) = event.event_type {
                                    // 录制模式：把鼠标操作记成表格里的行，
                                    // 同一个键快速点两下就并成一次双击。
                                    if listener_record_flag.load(Ordering::Acquire)
                                        && let Some(opt_type) = recorded_mouse_opt_type(button)
                                    {
                                        let (x, y, hwnd, title) = cursor_info();
                                        let now = Instant::now();
                                        let delay = take_recorded_delay(&mut last_recorded_at, now);
                                        if let Some(double_type) = recorded_double_opt_type(button)
                                            && is_double_click(last_click, button, now, x, y)
                                        {
                                            last_click = None;
                                            let _ = mouse_tx.try_send(
                                                HotkeyEvent::RecordedDoubleClick {
                                                    opt_type: double_type.label().to_string(),
                                                    x,
                                                    y,
                                                    hwnd,
                                                    title,
                                                    delay,
                                                },
                                            );
                                        } else {
                                            last_click = Some((button, now, x, y));
                                            let _ = mouse_tx.try_send(HotkeyEvent::RecordedInput {
                                                opt_cate: OptCate::鼠标,
                                                opt_type: opt_type.label().to_string(),
                                                x,
                                                y,
                                                hwnd,
                                                title,
                                                delay,
                                            });
                                        }
                                    }
                                } else if let EventType::Wheel { delta_y, .. } = event.event_type {
                                    // 录制模式：滚轮上滑/下滑各记一行。
                                    if listener_record_flag.load(Ordering::Acquire) && delta_y != 0
                                    {
                                        let (x, y, hwnd, title) = cursor_info();
                                        let opt_type = if delta_y > 0 {
                                            MouseOptType::滚轮上滑
                                        } else {
                                            MouseOptType::滚轮下滑
                                        };
                                        let delay = take_recorded_delay(
                                            &mut last_recorded_at,
                                            Instant::now(),
                                        );
                                        let _ = mouse_tx.try_send(HotkeyEvent::RecordedInput {
                                            opt_cate: OptCate::鼠标,
                                            opt_type: opt_type.label().to_string(),
                                            x,
                                            y,
                                            hwnd,
                                            title,
                                            delay,
                                        });
                                    }
                                } else if let EventType::KeyRelease(key) = event.event_type {
                                    modifiers.update(&key, false);

                                    // 录制模式：弹起也记一行「弹起 xxx」。
                                    // 被当成快捷键用掉的那次按键，弹起时不再写进表格。
                                    let plain_key = plain_key_label(&key);
                                    let consumed = hotkey_keys_down
                                        .iter()
                                        .position(|candidate| *candidate == plain_key);
                                    if let Some(position) = consumed {
                                        hotkey_keys_down.remove(position);
                                    } else if listener_record_flag.load(Ordering::Acquire) {
                                        let (x, y, hwnd, title) = cursor_info();
                                        let delay = take_recorded_delay(
                                            &mut last_recorded_at,
                                            Instant::now(),
                                        );
                                        let _ = mouse_tx.try_send(HotkeyEvent::RecordedInput {
                                            opt_cate: OptCate::键盘,
                                            opt_type: recorded_key_label(
                                                KeyAction::Up,
                                                caps_lock,
                                                &key,
                                            ),
                                            x,
                                            y,
                                            hwnd,
                                            title,
                                            delay,
                                        });
                                    }
                                } else if let EventType::KeyPress(key) = event.event_type {
                                    // 修饰键自己不成一次操作，只是记下状态供组合键使用。
                                    let is_modifier = modifiers.update(&key, true);
                                    if key == RdevKey::CapsLock {
                                        caps_lock = !caps_lock;
                                    }

                                    // 快捷键按“修饰键 + 主键”的规范名比对：Ctrl+F8 不会
                                    // 误触发只绑了 F8 的操作，大小写也不影响。
                                    let pressed = hotkey_key_label(&modifiers, &key);
                                    // 不带修饰键的键名，用来记「这个键被快捷键用掉了」。
                                    let plain_key = plain_key_label(&key);

                                    // 弹窗正在等新快捷键：这次按键只用来改绑定。
                                    if listener_capture_flag.load(Ordering::Acquire) {
                                        if !is_modifier {
                                            listener_capture_flag.store(false, Ordering::Release);
                                            let _ = mouse_tx
                                                .try_send(HotkeyEvent::HotkeyCaptured(pressed));
                                        }
                                        return;
                                    }

                                    // 开始/暂停、操作二连点、开始录制 是全局开关，优先级最高：
                                    //   1) 永远不受「监听热键」开关影响；
                                    //   2) 也永远排在「键盘行的输入框正在等着录键」前面——
                                    //      否则焦点留在某一行的操作方式上时，F8 会被当成字符写进那一行，
                                    //      结果就是「按了暂停却还在跑」。
                                    // 开始/暂停 还要当场把执行标志清掉：不等界面把事件处理完，
                                    // 正在跑的那一轮立刻停（界面慢或卡住都不影响停止）。
                                    if hotkey_pressed(
                                        &listener_hotkeys,
                                        HotkeyAction::ToggleRun,
                                        &pressed,
                                    ) {
                                        hotkey_keys_down.push(plain_key.clone());
                                        if listener_execution_flag.load(Ordering::Acquire) {
                                            listener_execution_flag.store(false, Ordering::Release);
                                        }
                                        let _ = mouse_tx.try_send(HotkeyEvent::ToggleExecution);
                                        return;
                                    }
                                    if hotkey_pressed(
                                        &listener_hotkeys,
                                        HotkeyAction::OptTwoRun,
                                        &pressed,
                                    ) {
                                        hotkey_keys_down.push(plain_key.clone());
                                        if listener_opt_two_flag.load(Ordering::Acquire) {
                                            listener_opt_two_flag.store(false, Ordering::Release);
                                        }
                                        let _ = mouse_tx.try_send(HotkeyEvent::ToggleOptTwo);
                                        return;
                                    }
                                    if hotkey_pressed(
                                        &listener_hotkeys,
                                        HotkeyAction::Record,
                                        &pressed,
                                    ) {
                                        hotkey_keys_down.push(plain_key.clone());
                                        let _ = mouse_tx.try_send(HotkeyEvent::ToggleRecording);
                                        return;
                                    }

                                    if let Some(row_ix) = *keyboard_input_row.lock().unwrap() {
                                        if !is_modifier {
                                            let _ = mouse_tx.try_send(HotkeyEvent::KeyboardInput(
                                                row_ix,
                                                keyboard_row_label(&modifiers, caps_lock, &key),
                                            ));
                                        }
                                        return;
                                    }

                                    // 录制中：按下的键记成一行「按下 xxx」，包括 CTRL/SHIFT 这些
                                    // 修饰键自己（它们的弹起会另记一行，这样按住的过程能还原）。
                                    if listener_record_flag.load(Ordering::Acquire) {
                                        let (x, y, hwnd, title) = cursor_info();
                                        let delay = take_recorded_delay(
                                            &mut last_recorded_at,
                                            Instant::now(),
                                        );
                                        let _ = mouse_tx.try_send(HotkeyEvent::RecordedInput {
                                            opt_cate: OptCate::键盘,
                                            opt_type: recorded_key_label(
                                                KeyAction::Down,
                                                caps_lock,
                                                &key,
                                            ),
                                            x,
                                            y,
                                            hwnd,
                                            title,
                                            delay,
                                        });
                                        return;
                                    }

                                    if is_modifier {
                                        return;
                                    }
                                    if !hotkey_listener_flag.load(Ordering::Acquire) {
                                        return;
                                    }
                                    if hotkey_pressed(
                                        &listener_hotkeys,
                                        HotkeyAction::MouseEvent,
                                        &pressed,
                                    ) {
                                        hotkey_keys_down.push(plain_key.clone());
                                        let (x, y, hwnd, title) = cursor_info();
                                        let _ = mouse_tx
                                            .try_send(HotkeyEvent::AddMouse(x, y, hwnd, title));
                                    } else if hotkey_pressed(
                                        &listener_hotkeys,
                                        HotkeyAction::KeyboardEvent,
                                        &pressed,
                                    ) {
                                        hotkey_keys_down.push(plain_key.clone());
                                        let (x, y, hwnd, title) = cursor_info();
                                        let _ = mouse_tx
                                            .try_send(HotkeyEvent::AddKeyboard(x, y, hwnd, title));
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
                                                if view.watch_hotkey
                                                    && view.hotkey_modal.is_none() =>
                                            {
                                                view.add_hotkey_row(
                                                    window,
                                                    cx,
                                                    x,
                                                    y,
                                                    hwnd,
                                                    title,
                                                    OptCate::鼠标,
                                                    None,
                                                    0,
                                                );
                                            }
                                            HotkeyEvent::AddKeyboard(x, y, hwnd, title)
                                                if view.watch_hotkey
                                                    && view.hotkey_modal.is_none() =>
                                            {
                                                view.add_hotkey_row(
                                                    window,
                                                    cx,
                                                    x,
                                                    y,
                                                    hwnd,
                                                    title,
                                                    OptCate::键盘,
                                                    None,
                                                    0,
                                                );
                                            }
                                            HotkeyEvent::ToggleRecording
                                                if view.hotkey_modal.is_none() =>
                                            {
                                                view.toggle_recording(cx);
                                            }
                                            HotkeyEvent::HotkeyCaptured(key) => {
                                                view.apply_captured_hotkey(key, cx);
                                            }
                                            HotkeyEvent::LoadConfig(path) => {
                                                // 成功不提示（不占地方），失败才在按钮下面显红字。
                                                if let Err(error) =
                                                    view.load_config(window, cx, &path)
                                                {
                                                    view.set_config_error(error, cx);
                                                } else {
                                                    view.clear_config_error(cx);
                                                }
                                            }
                                            HotkeyEvent::SavePicked(picked) => match picked {
                                                Ok(Some(path)) => {
                                                    match view.save_config_to(cx, &path) {
                                                        Ok(_) => view.clear_config_error(cx),
                                                        Err(error) => {
                                                            view.set_config_error(error, cx)
                                                        }
                                                    }
                                                }
                                                // 用户点了取消：什么都不做。
                                                Ok(None) => {}
                                                Err(error) => view.set_config_error(error, cx),
                                            },
                                            HotkeyEvent::LoadPicked(picked) => match picked {
                                                Ok(Some(path)) => {
                                                    match view.load_config(window, cx, &path) {
                                                        Ok(()) => view.clear_config_error(cx),
                                                        Err(error) => {
                                                            view.set_config_error(error, cx)
                                                        }
                                                    }
                                                }
                                                Ok(None) => {}
                                                Err(error) => view.set_config_error(error, cx),
                                            },
                                            HotkeyEvent::RecordedInput {
                                                opt_cate,
                                                opt_type,
                                                x,
                                                y,
                                                hwnd,
                                                title,
                                                delay,
                                            } => {
                                                view.add_hotkey_row(
                                                    window,
                                                    cx,
                                                    x,
                                                    y,
                                                    hwnd,
                                                    title,
                                                    opt_cate,
                                                    Some(opt_type),
                                                    delay,
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
                                            // 修改快捷键的弹窗开着的时候，全局快捷键不生效：
                                            // 否则刚按下的键会跑去做别的（比如开始点击）。
                                            HotkeyEvent::ToggleExecution
                                                if view.hotkey_modal.is_none() =>
                                            {
                                                view.toggle_execution(cx);
                                            }
                                            HotkeyEvent::ToggleOptTwo
                                                if view.hotkey_modal.is_none() =>
                                            {
                                                view.toggle_opt_two(cx);
                                            }
                                            HotkeyEvent::OptTwoFinished => {
                                                view.opt_two_running = false;
                                                cx.notify();
                                            }
                                            HotkeyEvent::RecordedDoubleClick {
                                                opt_type,
                                                x,
                                                y,
                                                hwnd,
                                                title,
                                                delay,
                                            } => {
                                                // 上一行就是这次单击，就地改成双击；
                                                // 万一没对上（比如中间插了别的操作）就补一行。
                                                let double_type =
                                                    MouseOptType::from_label(&opt_type);
                                                let upgraded = double_type.is_some_and(|kind| {
                                                    view.data_table.update(cx, |table, _| {
                                                        table
                                                            .delegate_mut()
                                                            .upgrade_last_mouse_row(kind, x, y)
                                                    })
                                                });
                                                let Some(double_type) = double_type else {
                                                    return;
                                                };
                                                if !upgraded {
                                                    view.add_hotkey_row(
                                                        window,
                                                        cx,
                                                        x,
                                                        y,
                                                        hwnd,
                                                        title,
                                                        OptCate::鼠标,
                                                        Some(opt_type),
                                                        delay,
                                                    );
                                                    return;
                                                }

                                                // 行数据改了，下拉框的显示也要跟着改。
                                                let select =
                                                    view.data_table.update(cx, |table, _| {
                                                        table
                                                            .delegate()
                                                            .opt_type_selects
                                                            .last()
                                                            .cloned()
                                                    });
                                                if let Some(select) = select {
                                                    select.update(cx, |select, cx| {
                                                        select.set_selected_index(
                                                            Some(IndexPath::new(
                                                                double_type.index(),
                                                            )),
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }
                                                cx.notify();
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

                        // 事件循环起来之后再把启动参数里的配置送进去读。
                        if let Some(path) = startup_config {
                            let _ = startup_config_tx.try_send(HotkeyEvent::LoadConfig(path));
                        }

                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}

/// 后台（SendMessage）双击路径的行为测试。
/// 用系统自带的 EDIT 控件当靶子：它的窗口类带 CS_DBLCLKS，
/// 而且双击在它身上的可观察结果很明确——选中光标下的整个单词，
/// 单击则只移动插入符、不选中任何文字。
#[cfg(test)]
mod double_click_tests {
    // 这里不用 `use super::*`：crate 根上 `use gpui::*` 会带进来 gpui 自己的
    // `test` 同名宏，glob 一进本模块就会把内建的 #[test] 属性遮蔽掉。
    use super::{
        CONFIG_VERSION, ConfigFile, Duration, HWND, HotkeyAction, HotkeyConfig, HotkeySettings,
        Instant, KeyAction, LEFT_DOUBLE_CLICK_LABEL, LPARAM, MAX_SCHEMES, MOUSE_MOVE_LABEL,
        MOUSE_OPT_TYPE_LABELS, ModifierState, MouseOptType, MyData, MyTableDelegate, OperationGap,
        OptCate, OptOneConfig, OptTwoConfig, RowConfig, SchemeConfig, SendMessageW, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WPARAM, class_accepts_double_click, config_file_stem, config_save_path,
        double_click_gap, double_click_time_ms, first_free_scheme_name, fuzzy_point,
        hotkey_key_label, hotkey_keystroke, is_double_click, is_left_double_click, keeps_clicking,
        keyboard_row_label, parse_config, plain_key_label, random_delay_between, random_seed,
        recorded_double_opt_type, recorded_key_label, recorded_mouse_opt_type, schemes_of,
        send_message_mouse, should_record_move, sleep_until_stopped, split_key_action,
        split_keyboard_combo, take_recorded_delay, waits_between_loops, write_config_file,
    };
    use rdev::{Button as RdevButton, Key as RdevKey};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, HMENU, SetWindowTextW, WINDOW_EX_STYLE, WS_POPUP,
    };
    use windows::core::w;

    // EM_GETSEL / EM_SETSEL 在 windows crate 的 Win32_UI_Controls 特性里，
    // 为两个稳定的消息号单独开一个特性不划算，这里直接写消息号。
    const EM_GETSEL: u32 = 0x00B0;
    const EM_SETSEL: u32 = 0x00B1;

    fn create_edit() -> HWND {
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                w!("hello world"),
                WS_POPUP,
                0,
                0,
                300,
                40,
                HWND::default(),
                HMENU::default(),
                HINSTANCE::default(),
                None,
            )
            .expect("创建 EDIT 控件失败")
        }
    }

    /// 当前选区的 (起点, 终点)。
    fn selection(hwnd: HWND) -> (usize, usize) {
        let result = unsafe { SendMessageW(hwnd, EM_GETSEL, WPARAM(0), LPARAM(0)) }.0 as usize;
        (result & 0xffff, (result >> 16) & 0xffff)
    }

    fn collapse_selection(hwnd: HWND) {
        unsafe {
            let _ = SendMessageW(hwnd, EM_SETSEL, WPARAM(0), LPARAM(0));
        }
    }

    /// 一个鼠标行，坐标落在第一个单词 “hello” 上。
    /// 一个鼠标行，坐标落在第一个单词 “hello” 上。
    fn mouse_row(opt_type: &str) -> MyData {
        MyData {
            x: 6,
            y: 10,
            win_hwnd: String::new(),
            opt_cate: OptCate::鼠标,
            opt_type: opt_type.to_string(),
            win_title: None,
            delay: 0,
            execute_time: 1,
        }
    }

    /// 一个只有行数据的表格委托，用来测与窗口无关的行逻辑。
    fn recorded_delegate(data: Vec<MyData>) -> MyTableDelegate {
        MyTableDelegate {
            data,
            schemes: vec![super::Scheme {
                name: super::SCHEME_NAMES[0].to_string(),
                rows: Vec::new(),
            }],
            active_scheme: 0,
            columns: Vec::new(),
            delay_inputs: Vec::new(),
            execute_time_inputs: Vec::new(),
            opt_type_inputs: Vec::new(),
            opt_cate_selects: Vec::new(),
            opt_type_selects: Vec::new(),
            delay_subscriptions: Vec::new(),
            execute_time_subscriptions: Vec::new(),
            opt_cate_subscriptions: Vec::new(),
            opt_type_subscriptions: Vec::new(),
            opt_type_select_subscriptions: Vec::new(),
            focused_keyboard_row: std::sync::Arc::new(std::sync::Mutex::new(None)),
            expected_keyboard_value: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    #[test]
    fn left_double_click_sends_a_double_click_sequence() {
        assert!(is_left_double_click(LEFT_DOUBLE_CLICK_LABEL));
        assert!(!is_left_double_click("左键单击"));
        assert!(!is_left_double_click("右键双击"));

        let gap = double_click_gap();
        assert!(gap >= Duration::from_millis(15));
        assert!(gap.as_millis() as u64 * 2 < double_click_time_ms());

        let hwnd = create_edit();
        unsafe {
            let _ = SetWindowTextW(hwnd, w!("hello world"));
        }
        assert!(
            class_accepts_double_click(hwnd),
            "EDIT 的窗口类应当带 CS_DBLCLKS"
        );

        // 左键单击：只移动插入符。
        send_message_mouse(hwnd, &mouse_row("左键单击"), (6, 10));
        let (click_start, click_end) = selection(hwnd);
        assert_eq!(click_start, click_end, "单击不应该选中任何文字");

        // 左键双击：选中光标下的整个单词。
        collapse_selection(hwnd);
        send_message_mouse(hwnd, &mouse_row(LEFT_DOUBLE_CLICK_LABEL), (6, 10));
        let (start, end) = selection(hwnd);
        assert!(end <= "hello world".len());
        // 只断言选中了光标下的那个词：合成的鼠标消息没有真实移动过光标，
        // EDIT 会把词后面的分隔符（空格）一起选进来，那属于控件自身的取舍。
        assert!(end > start, "左键双击应当选中文字");
        let selected = &"hello world"[start..end];
        assert_eq!(selected.trim_end(), "hello", "左键双击应当选中光标下的单词");

        // 只发两次普通按下（缺少 WM_LBUTTONDBLCLK）不构成双击，
        // 这正是 send_message_mouse 要按类样式分支的原因。
        collapse_selection(hwnd);
        let lparam = LPARAM((((10u32) << 16) | 6u32) as isize);
        unsafe {
            let _ = SendMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(1), lparam);
            let _ = SendMessageW(hwnd, WM_LBUTTONUP, WPARAM(0), lparam);
            let _ = SendMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(1), lparam);
            let _ = SendMessageW(hwnd, WM_LBUTTONUP, WPARAM(0), lparam);
        }
        let (plain_start, plain_end) = selection(hwnd);
        assert_eq!(
            plain_start, plain_end,
            "两次普通单击不是双击，不应该选中文字"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }

    /// 下拉框里的八种操作方式：文案、还原、双击标记都要对得上，
    /// 而且录制鼠标时写进表格的文案必须是这八种之一。
    #[test]
    fn every_mouse_opt_type_round_trips_through_its_label() {
        assert_eq!(MOUSE_OPT_TYPE_LABELS.len(), MouseOptType::ALL.len());
        assert_eq!(MOUSE_OPT_TYPE_LABELS[0].as_ref(), "左键单击");
        assert_eq!(MOUSE_OPT_TYPE_LABELS[1].as_ref(), LEFT_DOUBLE_CLICK_LABEL);
        assert_eq!(
            MOUSE_OPT_TYPE_LABELS.last().unwrap().as_ref(),
            MOUSE_MOVE_LABEL
        );

        for kind in MouseOptType::ALL {
            let label = kind.label().clone();
            let restored = MouseOptType::from_label(&label).expect("文案应当能还原成操作方式");
            assert_eq!(restored.index(), kind.index());
            assert_eq!(restored.label().as_ref(), label.as_ref());
        }

        // 前后空格不影响还原，老数据里的空值和不认识的文案则保持 None。
        assert_eq!(
            MouseOptType::from_label(" 左键双击 "),
            Some(MouseOptType::左键双击)
        );
        assert_eq!(MouseOptType::from_label(""), None);
        assert_eq!(MouseOptType::from_label("鼠标乱点"), None);

        assert!(MouseOptType::左键双击.is_double_click());
        assert!(MouseOptType::右键双击.is_double_click());
        assert!(!MouseOptType::左键单击.is_double_click());
        assert!(!MouseOptType::滚轮上滑.is_double_click());

        // 录制时按下鼠标键要落到真实存在的操作方式上。
        use rdev::Button as RdevButton;
        for button in [RdevButton::Left, RdevButton::Right, RdevButton::Middle] {
            let kind = recorded_mouse_opt_type(button).expect("三个鼠标键都该有对应操作");
            assert_eq!(MouseOptType::from_label(kind.label()), Some(kind));
        }
        assert_eq!(
            recorded_mouse_opt_type(RdevButton::Left),
            Some(MouseOptType::左键单击)
        );
        assert_eq!(
            recorded_mouse_opt_type(RdevButton::Right),
            Some(MouseOptType::右键单击)
        );
        assert_eq!(
            recorded_mouse_opt_type(RdevButton::Middle),
            Some(MouseOptType::滚轮单击)
        );
    }

    /// 快捷键键名：修饰键顺序固定、字母统一大写，
    /// 所以录制时写进去的绑定和按下时拼出来的键名一定对得上。
    #[test]
    fn hotkey_labels_cover_modifier_combinations() {
        let none = ModifierState::default();
        assert_eq!(hotkey_key_label(&none, &RdevKey::F8), "F8");
        assert_eq!(hotkey_key_label(&none, &RdevKey::KeyA), "A");

        let ctrl_shift = ModifierState {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        let shift_ctrl = ModifierState {
            shift: true,
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(
            hotkey_key_label(&ctrl_shift, &RdevKey::KeyA),
            "CTRL+SHIFT+A"
        );
        // 和按下的先后顺序无关。
        assert_eq!(
            hotkey_key_label(&ctrl_shift, &RdevKey::KeyA),
            hotkey_key_label(&shift_ctrl, &RdevKey::KeyA)
        );

        let all = ModifierState {
            ctrl: true,
            shift: true,
            alt: true,
            meta: true,
        };
        assert_eq!(
            hotkey_key_label(&all, &RdevKey::F1),
            "CTRL+SHIFT+ALT+WIN+F1"
        );

        // 绑定比对忽略大小写，但修饰键必须完全一致：Ctrl+F8 不该触发只绑了 F8 的操作。
        assert!(HotkeySettings::matches("CTRL+SHIFT+A", "ctrl+shift+a"));
        assert!(HotkeySettings::matches("F8", "f8"));
        assert!(!HotkeySettings::matches("F8", "CTRL+F8"));
        assert!(!HotkeySettings::matches("CTRL+F8", "F8"));
    }

    /// 组合键要能转成 gpui 显示的 Keystroke。
    #[test]
    fn hotkey_keystroke_understands_combinations() {
        let stroke = hotkey_keystroke("CTRL+SHIFT+F1");
        assert!(stroke.modifiers.control);
        assert!(stroke.modifiers.shift);
        assert!(!stroke.modifiers.alt);
        assert_eq!(stroke.key, "f1");

        let plain = hotkey_keystroke("F8");
        assert_eq!(plain.key, "f8");
        assert!(!plain.modifiers.control && !plain.modifiers.shift);

        let win = hotkey_keystroke("WIN+A");
        assert!(win.modifiers.platform);
        assert_eq!(win.key, "a");
    }

    /// 表格里键盘操作保存的键名：普通打字保持可读，组合键写成显式前缀。
    #[test]
    fn keyboard_row_labels_keep_typing_readable_and_combinations_explicit() {
        let none = ModifierState::default();
        assert_eq!(keyboard_row_label(&none, false, &RdevKey::KeyA), "a");
        assert_eq!(keyboard_row_label(&none, true, &RdevKey::KeyA), "A");
        assert_eq!(keyboard_row_label(&none, false, &RdevKey::F5), "F5");
        assert_eq!(keyboard_row_label(&none, false, &RdevKey::Num1), "1");

        let shift = ModifierState {
            shift: true,
            ..Default::default()
        };
        // 字母的 Shift 已经体现在大小写里，不再写成 SHIFT+。
        assert_eq!(keyboard_row_label(&shift, false, &RdevKey::KeyA), "A");
        // 功能键、数字和符号则要显式带上 SHIFT，否则执行时打不出 Shift+1 这种字符。
        assert_eq!(keyboard_row_label(&shift, false, &RdevKey::F5), "SHIFT+F5");
        assert_eq!(keyboard_row_label(&shift, false, &RdevKey::Num1), "SHIFT+1");

        let ctrl = ModifierState {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(keyboard_row_label(&ctrl, false, &RdevKey::KeyC), "CTRL+c");

        // 组合键里 Shift 必须单独写出来：CTRL+SHIFT+A 和 CTRL+A 是两个快捷键。
        let ctrl_shift = ModifierState {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        assert_eq!(
            keyboard_row_label(&ctrl_shift, false, &RdevKey::KeyA),
            "CTRL+SHIFT+a"
        );
        assert_eq!(
            keyboard_row_label(&ctrl_shift, false, &RdevKey::F5),
            "CTRL+SHIFT+F5"
        );

        let ctrl_alt = ModifierState {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        assert_eq!(
            keyboard_row_label(&ctrl_alt, false, &RdevKey::Delete),
            "CTRL+ALT+DELETE"
        );
    }

    /// 写进表格的键盘操作要能再拆回修饰键 + 主键，执行时才发得出组合键。
    #[test]
    fn keyboard_combos_split_back_into_modifiers_and_key() {
        assert_eq!(split_keyboard_combo("a"), (vec![], "a".to_string()));
        assert_eq!(
            split_keyboard_combo("CTRL+SHIFT+F5"),
            (vec![0x11, 0x10], "F5".to_string())
        );
        assert_eq!(
            split_keyboard_combo("CTRL+."),
            (vec![0x11], ".".to_string())
        );
        // 认不出来的前缀忽略掉，主键照发。
        assert_eq!(
            split_keyboard_combo("FOO+ENTER"),
            (vec![], "ENTER".to_string())
        );
        assert_eq!(split_keyboard_combo(""), (vec![], String::new()));
    }

    /// 录制的键盘行：按下、弹起各写一行，手填的键名仍然是「按一下」。
    #[test]
    fn recorded_key_rows_keep_press_and_release_apart() {
        // 文案：前缀 + 物理键名（修饰键自己有单独的行，所以键名里不带修饰键前缀）。
        assert_eq!(
            recorded_key_label(KeyAction::Down, false, &RdevKey::KeyA),
            "按下 a"
        );
        assert_eq!(
            recorded_key_label(KeyAction::Up, false, &RdevKey::KeyA),
            "按下 a".replace("按下", "弹起")
        );
        assert_eq!(
            recorded_key_label(KeyAction::Down, false, &RdevKey::ShiftLeft),
            "按下 SHIFT"
        );
        assert_eq!(
            recorded_key_label(KeyAction::Up, true, &RdevKey::KeyA),
            "弹起 A"
        );
        assert_eq!(
            recorded_key_label(KeyAction::Down, false, &RdevKey::F5),
            "按下 F5"
        );
        // 手填的一整个按键没有前缀。
        assert_eq!(
            recorded_key_label(KeyAction::Press, false, &RdevKey::KeyA),
            "a"
        );

        // 反过来能把动作拆回来。
        assert_eq!(split_key_action("按下 a"), (KeyAction::Down, "a"));
        assert_eq!(split_key_action("弹起 a"), (KeyAction::Up, "a"));
        assert_eq!(split_key_action("按下  CTRL "), (KeyAction::Down, "CTRL"));
        assert_eq!(split_key_action("a"), (KeyAction::Press, "a"));
        assert_eq!(split_key_action("CTRL+c"), (KeyAction::Press, "CTRL+c"));
        assert_eq!(split_key_action("弹起 F5"), (KeyAction::Up, "F5"));
    }

    /// 鼠标轨迹抽稀：间隔太短或没怎么动都不记。
    #[test]
    fn mouse_moves_are_thinned_out() {
        let start = Instant::now();

        // 第一条（录制开始后的第一次移动）总要记。
        assert!(should_record_move(None, start, 10.0, 10.0));

        let last = Some((start, 10.0, 10.0));
        // 时间没到、距离也不够：不记。
        assert!(!should_record_move(
            last,
            start + Duration::from_millis(5),
            11.0,
            11.0
        ));
        // 时间够久但基本没动：不记。
        assert!(!should_record_move(
            last,
            start + Duration::from_millis(100),
            11.0,
            11.0
        ));
        // 动得够多、时间也够：记。
        assert!(should_record_move(
            last,
            start + Duration::from_millis(30),
            40.0,
            10.0
        ));
        assert!(should_record_move(
            last,
            start + Duration::from_millis(30),
            10.0,
            40.0
        ));

        // 快捷键用掉的键名不带修饰键，方便弹起时对账。
        assert_eq!(plain_key_label(&RdevKey::KeyA), "A");
        assert_eq!(plain_key_label(&RdevKey::F10), "F10");
    }

    /// 录制的延迟：第一条是 0，之后每条都是距上一条真实过去的时间。
    #[test]
    fn recorded_rows_carry_the_gap_from_the_previous_one() {
        let start = Instant::now();
        let mut last = None;

        assert_eq!(take_recorded_delay(&mut last, start), 0, "第一条没有上一步");
        assert_eq!(
            take_recorded_delay(&mut last, start + Duration::from_millis(120)),
            120
        );
        // 紧接着的下一条只有几毫秒，就照实记几毫秒（而不是被抹成 0）。
        assert_eq!(
            take_recorded_delay(&mut last, start + Duration::from_millis(125)),
            5
        );
        assert_eq!(
            take_recorded_delay(&mut last, start + Duration::from_millis(1125)),
            1000
        );
    }

    /// 配置文件：存出来再读回去要一模一样。
    #[test]
    fn config_round_trips_through_json() {
        let config = ConfigFile {
            version: CONFIG_VERSION,
            saved_at: "2025-09-18 22:30:00".to_string(),
            opt_one: OptOneConfig {
                always_on_top: true,
                front_opt: true,
                fuzzy_click: true,
                random_delay: true,
                random_delay_min: "30".to_string(),
                random_delay_max: "90".to_string(),
                fuzzy_round: "5".to_string(),
                loop_count: "3".to_string(),
                loop_gap: "200".to_string(),
                opt_gap: "40".to_string(),
                opt_cate: 1,
                hide_when_choose: true,
                watch_hotkey: false,
                x: "100".to_string(),
                y: "200".to_string(),
                win_hwnd: "0x1234".to_string(),
                win_title: "记事本".to_string(),
            },
            opt_two: OptTwoConfig {
                click_gap: "30".to_string(),
                click_count: "9".to_string(),
            },
            hotkeys: HotkeyConfig {
                record: "F11".to_string(),
                toggle_run: "CTRL+F8".to_string(),
                mouse_event: "F6".to_string(),
                keyboard_event: "F7".to_string(),
                opt_two_run: "F9".to_string(),
            },
            schemes: Vec::new(),
            active_scheme: 0,
            rows: vec![
                RowConfig {
                    x: 100,
                    y: 200,
                    win_hwnd: "0x1234".to_string(),
                    opt_cate: "鼠标".to_string(),
                    opt_type: "左键双击".to_string(),
                    win_title: Some("记事本".to_string()),
                    delay: 120,
                    execute_time: 2,
                },
                RowConfig {
                    x: 0,
                    y: 0,
                    win_hwnd: "0x1234".to_string(),
                    opt_cate: "键盘".to_string(),
                    opt_type: "按下 CTRL".to_string(),
                    win_title: None,
                    delay: 0,
                    execute_time: 1,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&config).expect("能序列化");
        let restored: ConfigFile = serde_json::from_str(&json).expect("能反序列化");
        assert_eq!(restored, config);
        restored.validate().expect("自己存的配置必须能通过校验");
        assert_eq!(
            HotkeySettings::from(restored.hotkeys.clone()).toggle_run,
            "CTRL+F8"
        );
    }

    /// 校验：版本不对、操作类型/操作方式不对都要拦下来；缺省字段按默认值补。
    #[test]
    fn config_validation_rejects_bad_files() {
        // 不是本程序的 json：连 version 都没有。
        assert!(serde_json::from_str::<ConfigFile>(r#"{"foo": 1}"#).is_err());
        // 版本太新 / 为 0。
        let too_new: ConfigFile =
            serde_json::from_str(r#"{"version": 99}"#).expect("字段齐全就能解析");
        assert!(too_new.validate().is_err());
        let zero: ConfigFile = serde_json::from_str(r#"{"version": 0}"#).expect("能解析");
        assert!(zero.validate().is_err());

        // 只有版本号时其余字段用默认值补上，而且能通过校验（空表格）。
        let minimal: ConfigFile = serde_json::from_str(r#"{"version": 1}"#).expect("能解析");
        assert_eq!(minimal.opt_one.loop_count, "1");
        assert_eq!(minimal.opt_one.opt_gap, "20");
        assert_eq!(minimal.opt_two.click_gap, "20");
        assert_eq!(minimal.hotkeys.toggle_run, "F8");
        assert!(minimal.rows.is_empty());
        minimal.validate().expect("最小配置应当合法");

        // 行数据不合法：操作类型乱写、鼠标操作方式不认识、键盘行没写按键。
        let bad_cate: ConfigFile = serde_json::from_str(
            r#"{"version":1,"rows":[{"x":1,"y":2,"opt_cate":"触屏","opt_type":"点一下"}]}"#,
        )
        .expect("能解析");
        assert!(bad_cate.validate().is_err());

        let bad_type: ConfigFile = serde_json::from_str(
            r#"{"version":1,"rows":[{"x":1,"y":2,"opt_cate":"鼠标","opt_type":"乱点"}]}"#,
        )
        .expect("能解析");
        assert!(bad_type.validate().is_err());

        let bad_key: ConfigFile = serde_json::from_str(
            r#"{"version":1,"rows":[{"x":1,"y":2,"opt_cate":"键盘","opt_type":"  "}]}"#,
        )
        .expect("能解析");
        assert!(bad_key.validate().is_err());

        // execute_time 没写时按 1 补。
        let no_time: ConfigFile = serde_json::from_str(
            r#"{"version":1,"rows":[{"x":1,"y":2,"opt_cate":"鼠标","opt_type":"左键单击"}]}"#,
        )
        .expect("能解析");
        assert_eq!(no_time.rows[0].execute_time, 1);
        no_time.validate().expect("合法行");
    }

    /// 配置文件名要符合 iclick_config_年_月_日.json。
    #[test]
    fn config_file_name_uses_today() {
        let stem = config_file_stem();
        // iclick_config_YYYY_MM_DD 拆开是 5 段。
        let parts: Vec<&str> = stem.split('_').collect();
        assert_eq!(parts.len(), 5, "形如 iclick_config_2025_09_18：{stem}");
        assert_eq!(parts[0], "iclick");
        assert_eq!(parts[1], "config");
        assert_eq!(parts[2].len(), 4, "年份是四位：{stem}");
        assert_eq!(parts[3].len(), 2, "月份是两位：{stem}");
        assert_eq!(parts[4].len(), 2, "日期是两位：{stem}");
        for part in &parts[2..] {
            assert!(part.chars().all(|ch| ch.is_ascii_digit()), "{stem}");
        }
        assert!(
            config_save_path()
                .to_string_lossy()
                .ends_with(&format!("{stem}.json")),
            "保存路径要用这个名字"
        );
    }

    /// 配置文件写出来再读回去要一模一样（真的过一遍磁盘）。
    #[test]
    fn config_file_survives_a_disk_round_trip() {
        let config = ConfigFile {
            version: CONFIG_VERSION,
            saved_at: "2025-09-18 23:05:00".to_string(),
            opt_one: OptOneConfig {
                front_opt: true,
                fuzzy_click: true,
                fuzzy_round: "9".to_string(),
                loop_count: "2".to_string(),
                opt_gap: "45".to_string(),
                watch_hotkey: false,
                x: "77".to_string(),
                y: "88".to_string(),
                win_hwnd: "0x55".to_string(),
                ..OptOneConfig::default()
            },
            opt_two: OptTwoConfig {
                click_gap: "12".to_string(),
                click_count: "6".to_string(),
            },
            hotkeys: HotkeyConfig::default(),
            schemes: Vec::new(),
            active_scheme: 0,
            rows: vec![RowConfig {
                x: 5,
                y: 6,
                win_hwnd: "0x55".to_string(),
                opt_cate: "键盘".to_string(),
                opt_type: "按下 A".to_string(),
                win_title: None,
                delay: 250,
                execute_time: 3,
            }],
        };

        let path =
            std::env::temp_dir().join(format!("iclick_config_test_{}.json", std::process::id()));
        write_config_file(&config, &path).expect("能写文件");
        let text = std::fs::read_to_string(&path).expect("能读回文件");
        let _ = std::fs::remove_file(&path);

        let restored = parse_config(&text).expect("能解析校验");
        assert_eq!(restored, config);
        assert_eq!(restored.rows[0].delay, 250);
        assert_eq!(restored.rows[0].execute_time, 3);
    }

    /// 别人的 json / 内容不合法的 json 都要被挡下来。
    #[test]
    fn parse_config_rejects_foreign_and_broken_files() {
        assert!(
            parse_config("{\"hello\": \"world\"}").is_err(),
            "缺 version"
        );
        assert!(parse_config("not json at all").is_err(), "压根不是 json");
        assert!(
            parse_config(
                r#"{"version":1,"rows":[{"x":1,"y":2,"opt_cate":"鼠标","opt_type":"乱写"}]}"#
            )
            .is_err(),
            "操作方式不认识"
        );
        assert!(
            parse_config(r#"{"version":1,"rows":[]}"#).is_ok(),
            "空表格的配置是合法的"
        );
    }

    /// 多方案：配置里存了三个方案就读出三个，老配置（只有 rows）当成唯一的方案一。
    #[test]
    fn schemes_survive_saving_and_loading() {
        let scheme = |name: &str, x: usize| SchemeConfig {
            name: name.to_string(),
            rows: vec![RowConfig {
                x,
                y: x,
                win_hwnd: "0x1".to_string(),
                opt_cate: "鼠标".to_string(),
                opt_type: "左键单击".to_string(),
                win_title: None,
                delay: 10,
                execute_time: 1,
            }],
        };
        let config = ConfigFile {
            version: CONFIG_VERSION,
            saved_at: String::new(),
            opt_one: OptOneConfig::default(),
            opt_two: OptTwoConfig::default(),
            hotkeys: HotkeyConfig::default(),
            schemes: vec![
                scheme("方案一", 1),
                scheme("方案二", 2),
                scheme("方案三", 3),
            ],
            active_scheme: 2,
            rows: vec![],
        };
        config.validate().expect("三个方案是合法的");

        let json = serde_json::to_string_pretty(&config).expect("能序列化");
        let restored = parse_config(&json).expect("能读回来");
        assert_eq!(restored.schemes.len(), 3);
        assert_eq!(restored.active_scheme, 2);

        let schemes = schemes_of(&restored);
        assert_eq!(schemes.len(), 3);
        assert_eq!(schemes[0].name, "方案一");
        assert_eq!(schemes[2].name, "方案三");
        assert_eq!(schemes[1].rows[0].x, 2, "每个方案的表格内容各归各的");

        // 老配置：没有 schemes 字段，只有一份 rows。
        let old: ConfigFile = parse_config(
            r#"{"version":1,"rows":[{"x":9,"y":9,"win_hwnd":"0x1","opt_cate":"鼠标","opt_type":"左键单击"}]}"#,
        )
        .expect("老配置能读");
        let schemes = schemes_of(&old);
        assert_eq!(schemes.len(), 1, "老配置就是方案一");
        assert_eq!(schemes[0].name, "方案一");
        assert_eq!(schemes[0].rows[0].x, 9);

        // 方案名空着就按顺序补，超过 5 个方案要报错。
        let unnamed: ConfigFile =
            parse_config(r#"{"version":2,"schemes":[{"rows":[]},{"rows":[]}]}"#).expect("能解析");
        let schemes = schemes_of(&unnamed);
        assert_eq!(schemes[0].name, "方案一");
        assert_eq!(schemes[1].name, "方案二");

        let too_many = ConfigFile {
            version: CONFIG_VERSION,
            saved_at: String::new(),
            opt_one: OptOneConfig::default(),
            opt_two: OptTwoConfig::default(),
            hotkeys: HotkeyConfig::default(),
            schemes: (0..MAX_SCHEMES + 1)
                .map(|_| SchemeConfig {
                    name: String::new(),
                    rows: vec![],
                })
                .collect(),
            active_scheme: 0,
            rows: vec![],
        };
        assert!(too_many.validate().is_err(), "最多 5 个方案");

        // 某个方案里的行不合法，报错里要指明是哪个方案。
        // 这里用 serde 直接解析（parse_config 会顺手校验，拿不到错误文案）。
        let bad_row: ConfigFile = serde_json::from_str(
            r#"{"version":2,"schemes":[{"name":"方案二","rows":[{"x":1,"y":2,"opt_cate":"鼠标","opt_type":"瞎写"}]}]}"#,
        )
        .expect("能解析");
        let error = bad_row.validate().expect_err("应当报错");
        assert!(error.contains("方案二"), "报错要指明方案：{error}");
    }

    /// 随机延迟：取出来的值一定落在范围内；范围填反了照样能用；上下限相等就是定值。
    #[test]
    fn random_delay_stays_inside_the_range() {
        let mut seed = 0x1234_5678_9abc_def0_u64;
        for _ in 0..200 {
            let value = random_delay_between(&mut seed, 100, 200);
            assert!((100..=200).contains(&value), "越界了：{value}");
        }
        // 上下限相等 → 就是那个数
        assert_eq!(random_delay_between(&mut seed, 77, 77), 77);
        // 范围填反 → 按小的那个当唯一下限（max <= min 时取 min）
        assert_eq!(random_delay_between(&mut seed, 30, 10), 30);
        // 0..=0 是合法的（不加延迟）
        assert_eq!(random_delay_between(&mut seed, 0, 0), 0);
        // 多次取值不应该永远一样
        let values: Vec<usize> = (0..50)
            .map(|_| random_delay_between(&mut seed, 0, 1000))
            .collect();
        assert!(
            values.iter().any(|value| *value != values[0]),
            "随机延迟应当有变化：{values:?}"
        );
    }

    /// 新方案取名：删掉中间那个方案后，再添加应该补上空出来的名字，而不是撞名。
    #[test]
    fn new_scheme_names_fill_the_gaps() {
        assert_eq!(first_free_scheme_name(&[]), "方案一");
        assert_eq!(first_free_scheme_name(&["方案一"]), "方案二");
        assert_eq!(first_free_scheme_name(&["方案一", "方案三"]), "方案二");
        assert_eq!(
            first_free_scheme_name(&["方案一", "方案二", "方案四", "方案五"]),
            "方案三"
        );
        assert_eq!(
            first_free_scheme_name(&["方案一", "方案二", "方案三", "方案四", "方案五"]),
            "方案一",
            "五个都占满时兜底返回方案一（调用方本来就该拦住）"
        );
    }

    /// 修饰键自己不成一次操作，只用来拼组合键。
    #[test]
    fn modifier_state_tracks_keys_and_folds_into_labels() {
        let mut modifiers = ModifierState::default();
        assert!(modifiers.update(&RdevKey::ControlLeft, true));
        assert!(modifiers.update(&RdevKey::ShiftRight, true));
        assert!(!modifiers.update(&RdevKey::KeyA, true));
        assert_eq!(hotkey_key_label(&modifiers, &RdevKey::KeyA), "CTRL+SHIFT+A");

        assert!(modifiers.update(&RdevKey::ControlLeft, false));
        assert!(!modifiers.ctrl);
        assert!(modifiers.update(&RdevKey::AltGr, true));
        assert!(modifiers.alt);
    }

    /// 录制时把两次快速点击并成双击的判定。
    #[test]
    fn double_click_needs_same_button_time_and_place() {
        let now = Instant::now();
        let previous = Some((RdevButton::Left, now, 100.0, 200.0));

        assert!(is_double_click(
            previous,
            RdevButton::Left,
            now + Duration::from_millis(50),
            101.0,
            201.0
        ));
        // 换了键、超时、位置跑远了都不算双击。
        assert!(!is_double_click(
            previous,
            RdevButton::Right,
            now + Duration::from_millis(50),
            100.0,
            200.0
        ));
        assert!(!is_double_click(
            previous,
            RdevButton::Left,
            now + Duration::from_millis(double_click_time_ms() + 50),
            100.0,
            200.0
        ));
        assert!(!is_double_click(
            previous,
            RdevButton::Left,
            now + Duration::from_millis(50),
            160.0,
            200.0
        ));
        assert!(!is_double_click(None, RdevButton::Left, now, 100.0, 200.0));

        assert_eq!(
            recorded_double_opt_type(RdevButton::Left),
            Some(MouseOptType::左键双击)
        );
        assert_eq!(
            recorded_double_opt_type(RdevButton::Right),
            Some(MouseOptType::右键双击)
        );
        // 中键没有双击的说法，两次中键就是两行滚轮单击。
        assert_eq!(recorded_double_opt_type(RdevButton::Middle), None);

        // 双击 → 单击的对应关系，录制合并时用它认上一行。
        assert_eq!(
            MouseOptType::左键双击.single_click(),
            Some(MouseOptType::左键单击)
        );
        assert_eq!(
            MouseOptType::右键双击.single_click(),
            Some(MouseOptType::右键单击)
        );
        assert_eq!(MouseOptType::左键单击.single_click(), None);
        assert_eq!(MouseOptType::滚轮单击.single_click(), None);
    }

    /// 录制时把第二下点击并进上一行：只有“同一位置的同一次单击”才并得动。
    #[test]
    fn recorded_double_click_upgrades_the_matching_row() {
        let mut delegate = recorded_delegate(vec![mouse_row("左键单击")]);
        assert!(
            delegate.upgrade_last_mouse_row(MouseOptType::左键双击, 6.0, 10.0),
            "同一位置、同一按键的单击行应当能就地升级"
        );
        assert_eq!(delegate.data[0].opt_type, "左键双击");

        // 位置差超过了系统双击范围就不并，宁可变回两行。
        let mut delegate = recorded_delegate(vec![mouse_row("左键单击")]);
        assert!(!delegate.upgrade_last_mouse_row(MouseOptType::左键双击, 100.0, 10.0));
        assert_eq!(delegate.data[0].opt_type, "左键单击");

        // 上一行是别的按键/别的类型（比如中间夹了一次滚轮）也不并。
        let mut delegate = recorded_delegate(vec![mouse_row("滚轮上滑")]);
        assert!(!delegate.upgrade_last_mouse_row(MouseOptType::左键双击, 6.0, 10.0));
        assert_eq!(delegate.data[0].opt_type, "滚轮上滑");

        // 上一行是键盘操作、或者干脆没有行，都并不了。
        let mut keyboard_row = mouse_row("a");
        keyboard_row.opt_cate = OptCate::键盘;
        let mut delegate = recorded_delegate(vec![keyboard_row]);
        assert!(!delegate.upgrade_last_mouse_row(MouseOptType::左键双击, 6.0, 10.0));
        let mut delegate = recorded_delegate(vec![]);
        assert!(!delegate.upgrade_last_mouse_row(MouseOptType::左键双击, 6.0, 10.0));

        // 右键同理。
        let mut delegate = recorded_delegate(vec![mouse_row("右键单击")]);
        assert!(delegate.upgrade_last_mouse_row(MouseOptType::右键双击, 7.0, 10.0));
        assert_eq!(delegate.data[0].opt_type, "右键双击");
    }

    /// 模糊点击：以行坐标为中心，在 ±模糊范围 里随机取点。
    #[test]
    fn fuzzy_point_stays_inside_the_range() {
        let mut state = 0x1234_5678_9abc_def1u64;

        // 范围 0 就是不模糊，原样返回。
        assert_eq!(fuzzy_point(100, 100, 0, &mut state), (100, 100));

        // 范围 5、中心 (100, 100)：x、y 都落在 95~105，而且确实在变。
        let mut seen_min = (usize::MAX, usize::MAX);
        let mut seen_max = (0usize, 0usize);
        let mut distinct = std::collections::HashSet::new();
        for _ in 0..2000 {
            let (x, y) = fuzzy_point(100, 100, 5, &mut state);
            assert!((95..=105).contains(&x), "x={x} 超出了 95~105");
            assert!((95..=105).contains(&y), "y={y} 超出了 95~105");
            seen_min = (seen_min.0.min(x), seen_min.1.min(y));
            seen_max = (seen_max.0.max(x), seen_max.1.max(y));
            distinct.insert((x, y));
        }
        assert_eq!(seen_min, (95, 95), "范围下界应当取得到");
        assert_eq!(seen_max, (105, 105), "范围上界应当取得到");
        assert!(distinct.len() > 50, "点位要随机，不能老是同一个");

        // 贴着左上角的行不会取到负坐标。
        for _ in 0..200 {
            let (x, y) = fuzzy_point(1, 0, 5, &mut state);
            assert!(x <= 6);
            assert!(y <= 5);
        }

        // 同一个种子给同一串点位，换个种子就换一串。
        let mut a = 42u64;
        let mut b = 42u64;
        let mut c = 43u64;
        let first: Vec<_> = (0..5).map(|_| fuzzy_point(100, 100, 5, &mut a)).collect();
        let same: Vec<_> = (0..5).map(|_| fuzzy_point(100, 100, 5, &mut b)).collect();
        let other: Vec<_> = (0..5).map(|_| fuzzy_point(100, 100, 5, &mut c)).collect();
        assert_eq!(first, same);
        assert_ne!(first, other);
        assert_ne!(random_seed(), 0, "种子不能是 0");
    }

    /// 循环间隔只在“还有下一轮”的时候等，最后一轮结束不用空等。
    #[test]
    fn loop_gap_only_waits_between_rounds() {
        // 循环 3 次：前两轮之后要等。
        assert!(waits_between_loops(3, 1));
        assert!(waits_between_loops(3, 2));
        assert!(!waits_between_loops(3, 3), "最后一轮结束后不该再等");

        // 只循环 1 次：根本没有下一轮。
        assert!(!waits_between_loops(1, 1));

        // 循环次数 0 表示无限次，每轮之间都要等。
        assert!(waits_between_loops(0, 1));
        assert!(waits_between_loops(0, 99));
    }

    /// 五个快捷键（含操作二的开始/停止）默认值互不冲突。
    #[test]
    fn hotkey_defaults_cover_opt_two_without_clashes() {
        assert_eq!(HotkeyAction::ALL.len(), 5);
        assert_eq!(HotkeyAction::OptTwoRun.default_key(), "F9");

        let settings = HotkeySettings::default();
        let mut seen = std::collections::HashSet::new();
        for action in HotkeyAction::ALL {
            let key = settings.get(action).clone();
            assert!(!key.trim().is_empty(), "{action:?} 应当有默认键");
            assert!(seen.insert(key.clone()), "默认键 {key} 被两个操作同时占用");
        }

        // 改完绑定之后，冲突检测要认得出是操作二占了别人的键。
        let mut settings = HotkeySettings::default();
        settings.set(HotkeyAction::OptTwoRun, "F8");
        assert_eq!(settings.get(HotkeyAction::OptTwoRun), "F8");
        assert_eq!(
            settings.conflict(HotkeyAction::OptTwoRun, "F8"),
            Some(HotkeyAction::ToggleRun)
        );
    }

    /// 操作二连点：次数 0 一直点，次数 N 点满就停，按了停止立刻不点。
    #[test]
    fn opt_two_clicks_until_stopped_or_counted_out() {
        assert!(keeps_clicking(true, 0, 0));
        assert!(keeps_clicking(true, 0, 9999));

        assert!(keeps_clicking(true, 3, 0));
        assert!(keeps_clicking(true, 3, 2));
        assert!(!keeps_clicking(true, 3, 3), "次数用完就该停");

        assert!(!keeps_clicking(false, 0, 0), "按了停止就不再点");
        assert!(!keeps_clicking(false, 3, 1));
    }

    /// 循环间隔期间点暂停，等待要立刻结束，而不是把整段间隔睡完。
    #[test]
    fn interruptible_wait_stops_early() {
        let running = Arc::new(AtomicBool::new(true));
        let start = Instant::now();
        assert!(sleep_until_stopped(&running, Duration::from_millis(60)));
        assert!(
            start.elapsed() >= Duration::from_millis(50),
            "正常运行时要等满间隔"
        );

        running.store(false, Ordering::Release);
        let start = Instant::now();
        assert!(!sleep_until_stopped(&running, Duration::from_millis(500)));
        assert!(
            start.elapsed() < Duration::from_millis(200),
            "暂停后不该继续等下去"
        );
    }

    /// 操作间隔：第一个操作不等，之后每个操作前都要等；填 0 就完全不等待。
    #[test]
    fn operation_gap_skips_the_first_wait() {
        let mut gap = OperationGap::new(20);
        assert_eq!(gap.wait_before_operation(), 0, "整场执行的第一个操作不用等");
        assert_eq!(gap.wait_before_operation(), 20);
        assert_eq!(gap.wait_before_operation(), 20);

        let mut none = OperationGap::new(0);
        assert_eq!(none.wait_before_operation(), 0);
        assert_eq!(none.wait_before_operation(), 0);
    }

    /// 传进去的点位要真的用在这一击上——模糊点击就是靠这个参数生效的。
    #[test]
    fn mouse_click_uses_the_given_point() {
        let hwnd = create_edit();
        unsafe {
            let _ = SetWindowTextW(hwnd, w!("hello world"));
        }

        send_message_mouse(hwnd, &mouse_row("左键单击"), (2, 10));
        let (near_start, near_end) = selection(hwnd);
        assert_eq!(near_start, near_end, "单击不应该选中文字");

        collapse_selection(hwnd);
        send_message_mouse(hwnd, &mouse_row("左键单击"), (40, 10));
        let (far_start, far_end) = selection(hwnd);
        assert_eq!(far_start, far_end, "单击不应该选中文字");
        assert!(
            far_start > near_start,
            "点位不同，插入符位置就该不同：{near_start} vs {far_start}"
        );

        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}
