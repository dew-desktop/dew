use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmEnableBlurBehindWindow, DWM_BB_ENABLE, DWM_BLURBEHIND};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn, UpdateWindow, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct NativeWindow {
    pub hwnd: HWND,
    pub width: u16,
    pub height: u16,
    pub is_dragging: bool,
    pub drag_start_cursor: (i32, i32),
    pub drag_start_pos: (i32, i32),
}

static mut GLOBAL_WINDOW_STATE: Option<*mut NativeWindow> = None;

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let mut cursor_pt = windows::Win32::Foundation::POINT::default();
            let _ = GetCursorPos(&mut cursor_pt);
            let mut window_rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut window_rect);

            if let Some(state_ptr) = GLOBAL_WINDOW_STATE {
                let state = &mut *state_ptr;
                state.is_dragging = true;
                state.drag_start_cursor = (cursor_pt.x, cursor_pt.y);
                state.drag_start_pos = (window_rect.left, window_rect.top);
                let _ = SetCapture(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(state_ptr) = GLOBAL_WINDOW_STATE {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    let mut cursor_pt = windows::Win32::Foundation::POINT::default();
                    let _ = GetCursorPos(&mut cursor_pt);
                    let dx = cursor_pt.x - state.drag_start_cursor.0;
                    let dy = cursor_pt.y - state.drag_start_cursor.1;

                    let new_x = state.drag_start_pos.0 + dx;
                    let new_y = state.drag_start_pos.1 + dy;

                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        new_x,
                        new_y,
                        state.width as i32,
                        state.height as i32,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(state_ptr) = GLOBAL_WINDOW_STATE {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    state.is_dragging = false;
                    let _ = ReleaseCapture();
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl NativeWindow {
    pub fn new(title: &str, width: u16, height: u16) -> Result<Self, String> {
        unsafe {
            let hinstance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
            let class_name = w!("DewHUDWindowClass");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: HINSTANCE(hinstance.0),
                hCursor: LoadCursorW(None, IDC_ARROW).map_err(|e| e.to_string())?,
                hbrBackground: HBRUSH(std::ptr::null_mut()),
                lpszClassName: class_name,
                ..Default::default()
            };

            let _ = RegisterClassExW(&wnd_class);

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let pos_x = (screen_w - width as i32) / 2;
            let pos_y = 60;

            let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name,
                PCWSTR(title_wide.as_ptr()),
                WS_POPUP | WS_VISIBLE,
                pos_x,
                pos_y,
                width as i32,
                height as i32,
                None,
                None,
                Some(HINSTANCE(hinstance.0)),
                None,
            ).map_err(|e| e.to_string())?;

            // Hardware Window Region Clipping
            let rgn = CreateRoundRectRgn(0, 0, width as i32 + 1, height as i32 + 1, 28, 28);
            let _ = SetWindowRgn(hwnd, Some(rgn), true);

            // Windows DWM Blur
            let blur = DWM_BLURBEHIND {
                dwFlags: DWM_BB_ENABLE,
                fEnable: true.into(),
                hRgnBlur: Default::default(),
                fTransitionOnMaximized: false.into(),
            };
            let _ = DwmEnableBlurBehindWindow(hwnd, &blur);

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            let mut window = Self {
                hwnd,
                width,
                height,
                is_dragging: false,
                drag_start_cursor: (0, 0),
                drag_start_pos: (pos_x, pos_y),
            };

            GLOBAL_WINDOW_STATE = Some(&mut window as *mut NativeWindow);

            Ok(window)
        }
    }
}
