use std::sync::{Arc, Mutex};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmEnableBlurBehindWindow, DWM_BB_ENABLE, DWM_BLURBEHIND};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn, UpdateWindow, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
pub struct WindowSharedState {
    pub is_dragging: bool,
    pub drag_start_cursor: (i32, i32),
    pub drag_start_pos: (i32, i32),
    pub pending_clicks: Vec<(i32, i32)>,
    pub width: u16,
    pub height: u16,
}

pub type SharedWindowState = Arc<Mutex<WindowSharedState>>;

static mut GLOBAL_SHARED_STATE: Option<SharedWindowState> = None;

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

            if let Some(ref shared) = GLOBAL_SHARED_STATE {
                let mut state = shared.lock().unwrap();
                state.is_dragging = true;
                state.drag_start_cursor = (cursor_pt.x, cursor_pt.y);
                state.drag_start_pos = (window_rect.left, window_rect.top);
                let _ = SetCapture(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(ref shared) = GLOBAL_SHARED_STATE {
                let state = shared.lock().unwrap();
                if state.is_dragging {
                    let mut cursor_pt = windows::Win32::Foundation::POINT::default();
                    let _ = GetCursorPos(&mut cursor_pt);
                    let dx = cursor_pt.x - state.drag_start_cursor.0;
                    let dy = cursor_pt.y - state.drag_start_cursor.1;

                    if dx.abs() > 3 || dy.abs() > 3 {
                        let new_x = state.drag_start_pos.0 + dx;
                        let new_y = state.drag_start_pos.1 + dy;
                        let (w, h) = (state.width as i32, state.height as i32);

                        let _ = SetWindowPos(
                            hwnd,
                            Some(HWND_TOPMOST),
                            new_x,
                            new_y,
                            w,
                            h,
                            SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(ref shared) = GLOBAL_SHARED_STATE {
                let mut state = shared.lock().unwrap();
                if state.is_dragging {
                    state.is_dragging = false;
                    let _ = ReleaseCapture();

                    let mut cursor_pt = windows::Win32::Foundation::POINT::default();
                    let _ = GetCursorPos(&mut cursor_pt);
                    let dx = cursor_pt.x - state.drag_start_cursor.0;
                    let dy = cursor_pt.y - state.drag_start_cursor.1;

                    if dx.abs() <= 6 && dy.abs() <= 6 {
                        let mut window_rect = RECT::default();
                        let _ = GetWindowRect(hwnd, &mut window_rect);
                        let local_x = cursor_pt.x - window_rect.left;
                        let local_y = cursor_pt.y - window_rect.top;
                        state.pending_clicks.push((local_x, local_y));
                    }
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub struct NativeWindow {
    pub hwnd: HWND,
    pub width: u16,
    pub height: u16,
    pub shared: SharedWindowState,
}

impl NativeWindow {
    pub fn new(title: &str, width: u16, height: u16) -> Result<Self, String> {
        let shared = Arc::new(Mutex::new(WindowSharedState {
            width,
            height,
            ..Default::default()
        }));

        unsafe {
            GLOBAL_SHARED_STATE = Some(Arc::clone(&shared));

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

            let rgn = CreateRoundRectRgn(0, 0, width as i32 + 1, height as i32 + 1, 28, 28);
            let _ = SetWindowRgn(hwnd, Some(rgn), true);

            let blur = DWM_BLURBEHIND {
                dwFlags: DWM_BB_ENABLE,
                fEnable: true.into(),
                hRgnBlur: Default::default(),
                fTransitionOnMaximized: false.into(),
            };
            let _ = DwmEnableBlurBehindWindow(hwnd, &blur);

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            Ok(Self {
                hwnd,
                width,
                height,
                shared,
            })
        }
    }

    pub fn drain_clicks(&self) -> Vec<(i32, i32)> {
        let mut state = self.shared.lock().unwrap();
        std::mem::take(&mut state.pending_clicks)
    }
}
