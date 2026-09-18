$ErrorActionPreference = 'Stop'

$lines = Get-Content -LiteralPath 'src\main.rs' -Encoding utf8

$header = @'
use crate::{HotkeyEvent, IClick};

use gpui::*;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::kbd::Kbd;
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::radio::RadioGroup;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState};
use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_component::tooltip::Tooltip;
use gpui_component::*;
use gpui_component::{Icon, IconName};
use rdev::{Button as RdevButton, Event, EventType, Key as RdevKey, listen};
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::Duration;
use std::vec;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MAPVK_VK_TO_VSC, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, MapVirtualKeyW,
    SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowTextW, HWND_NOTOPMOST,
    HWND_TOPMOST, IDC_CROSS, LoadCursorW, OCR_NORMAL, SPI_SETCURSORS, SPIF_SENDCHANGE, SWP_NOMOVE,
    SWP_NOSIZE, SW_HIDE, SW_SHOW, SendMessageW, SetCursorPos, SetForegroundWindow, SetSystemCursor,
    SetWindowPos, ShowWindow, SystemParametersInfoW, WM_CHAR, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WindowFromPoint,
};
'@

Set-Content -LiteralPath 'src\opt_one.rs' -Value $header -Encoding utf8

function Append($items) {
    if ($null -ne $items -and $items.Count -gt 0) {
        Add-Content -LiteralPath 'src\opt_one.rs' -Value $items -Encoding utf8
    }
}

# 39-88   set_selection_cursor, window_info_at, cursor_info
Append $lines[38..87]
# 139-227 OptCate, MouseOptType, labels, MyData, SelectItem impls
Append $lines[138..226]
# 229-243 MyTableDelegate struct
Append $lines[228..242]
# 245-506 impl MyTableDelegate { new, get_row_opt_cate, add_row }
Append $lines[244..505]
# 508-649 impl TableDelegate
Append $lines[507..648]
# 651-693 impl MyTableDelegate { remove_row }
Append $lines[650..692]
# 698-859 impl IClick { toggle_execution, hide_for_selection, show_after_selection, add_hotkey_row, finish_selection }
Append $lines[697..858]
# 861-1185 parse_hwnd, keyboard fns, send fns
Append $lines[860..1184]

Add-Content -LiteralPath 'src\opt_one.rs' -Value '' -Encoding utf8

$renderSig = @'
impl IClick {
    pub(crate) fn render_opt_one(&mut self, cx: &mut Context<Self>) -> Div {
'@
Add-Content -LiteralPath 'src\opt_one.rs' -Value $renderSig -Encoding utf8
# 1208-1774 render body (active_tab == 0 branch)
Append $lines[1207..1773]
Add-Content -LiteralPath 'src\opt_one.rs' -Value '    }' -Encoding utf8
Add-Content -LiteralPath 'src\opt_one.rs' -Value '}' -Encoding utf8
Add-Content -LiteralPath 'src\opt_one.rs' -Value '' -Encoding utf8

$handleEvent = @'
impl IClick {
    fn handle_event(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        event: HotkeyEvent,
    ) {
        match event {
            HotkeyEvent::MouseMove(x, y, hwnd, title) if self.is_choosing => {
                self.x_input.update(cx, |input, cx| {
                    input.set_value(&x.to_string(), window, cx);
                });
                self.y_input.update(cx, |input, cx| {
                    input.set_value(&y.to_string(), window, cx);
                });
                self.win_hwnd_input.update(cx, |input, cx| {
                    input.set_value(&hwnd, window, cx);
                });
                self.win_title_input.update(cx, |input, cx| {
                    input.set_value(&title, window, cx);
                });
                cx.notify();
            }
            HotkeyEvent::MouseReleased if self.is_choosing => {
                self.finish_selection(window, cx);
            }
            HotkeyEvent::AddMouse(x, y, hwnd, title) if self.watch_hotkey => {
                self.add_hotkey_row(window, cx, x, y, hwnd, title, OptCate::鼠标);
            }
            HotkeyEvent::AddKeyboard(x, y, hwnd, title) if self.watch_hotkey => {
                self.add_hotkey_row(window, cx, x, y, hwnd, title, OptCate::键盘);
            }
            HotkeyEvent::KeyboardInput(row_ix, value) => {
                self.data_table.update(cx, |table, cx| {
                    *table.delegate().expected_keyboard_value.lock().unwrap() =
                        Some((row_ix, value.clone()));
                    if let Some(input) = table.delegate().opt_type_inputs.get(row_ix) {
                        input.update(cx, |input, cx| {
                            input.set_value(&value, window, cx);
                        });
                    }
                });
            }
            HotkeyEvent::ToggleExecution => {
                self.toggle_execution(cx);
            }
            HotkeyEvent::ExecutionFinished => {
                self.is_running = false;
                cx.notify();
            }
            _ => {}
        }
    }
}
'@
Add-Content -LiteralPath 'src\opt_one.rs' -Value $handleEvent -Encoding utf8
Add-Content -LiteralPath 'src\opt_one.rs' -Value '' -Encoding utf8

$spawnEngine = @'
pub(crate) fn spawn_engine(
    view: &Entity<IClick>,
    window: &mut Window,
    cx: &mut App,
    mouse_rx: async_channel::Receiver<HotkeyEvent>,
) {
    let choosing_flag = view.read(cx).choosing_flag.clone();
    let hotkey_flag = view.read(cx).hotkey_flag.clone();
    let mouse_tx = view.read(cx).execution_event_tx.clone();
    let keyboard_input_row = view
        .read(cx)
        .data_table
        .read(cx)
        .delegate()
        .focused_keyboard_row
        .clone();

    std::thread::spawn(move || {
        let mut shift_pressed = false;
        let mut caps_lock = false;
        let callback = move |event: Event| {
            if choosing_flag.load(Ordering::Acquire)
                && let EventType::MouseMove { x: _, y: _ } = event.event_type
            {
                let (relative_x, relative_y, hwnd, title) = cursor_info();
                let _ =
                    mouse_tx.try_send(HotkeyEvent::MouseMove(relative_x, relative_y, hwnd, title));
            } else if choosing_flag.load(Ordering::Acquire)
                && let EventType::ButtonRelease(RdevButton::Left) = event.event_type
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
                if !hotkey_flag.load(Ordering::Acquire) {
                    return;
                }
                let (x, y, hwnd, title) = cursor_info();
                match key {
                    RdevKey::F6 => {
                        let _ = mouse_tx.try_send(HotkeyEvent::AddMouse(x, y, hwnd, title));
                    }
                    RdevKey::F7 => {
                        let _ = mouse_tx.try_send(HotkeyEvent::AddKeyboard(x, y, hwnd, title));
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
                    mouse_view.update(cx, |view, cx| {
                        view.handle_event(window, cx, event);
                    });
                });
            }
        })
        .detach();
}
'@
Add-Content -LiteralPath 'src\opt_one.rs' -Value $spawnEngine -Encoding utf8

Write-Output "opt_one.rs written"
