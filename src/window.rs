use windows::{
    core::{Result, Error, PCSTR},
    s,
    Win32::{
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
        UI::Input::KeyboardAndMouse::VK_F4,
        Foundation::{RECT, HWND, LPARAM, LRESULT, WPARAM},
    }
};

pub const WINDOW_CLASS_NAME: PCSTR = s!("win32.Window");

pub struct Win32OffscreenBuffer {
    // Pixels always are 32-bits wide, Memory Order BB GG RR XX
    pub info: BITMAPINFO,
    pub bits: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

pub struct Window {
    pub handle: HWND,
    pub buffer: Win32OffscreenBuffer,
    pub window_running: bool,
    pub refresh_rate: i32,
}
pub trait CheckHandle: Sized {
    fn ok(self) -> Result<Self>;
}
impl CheckHandle for HWND {
    fn ok(self) -> Result<Self> {
        if self.0 == 0 {
            Err(Error::from_win32())
        } else {
            Ok(self)
        }
    }
}

pub unsafe extern "system" fn win32_main_window_callback(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => {
            let cs = lparam.0 as *const CREATESTRUCTW;
            let this = (*cs).lpCreateParams as *mut Window;
            (*this).handle = window;

            SetWindowLongPtrA(window, GWLP_USERDATA, this as _);
        }
        WM_CLOSE | WM_DESTROY => {
            let this = GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Window;
            if let Some(this) = this.as_mut() {
                this.window_running = false;
            }
        }
        WM_SYSKEYDOWN => println!("Keyboard input came in through a non-dispatch message"),
        WM_SYSKEYUP => println!("Keyboard input came in through a non-dispatch message"),
        WM_KEYDOWN => println!("Keyboard input came in through a non-dispatch message"),
        WM_KEYUP => println!("Keyboard input came in through a non-dispatch message"),
        WM_PAINT => {
            let this = GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Window;
            if let Some(this) = this.as_mut() {
                let mut paint: PAINTSTRUCT = Default::default();
                let device_context = BeginPaint(window, &mut paint);
                win32_display_buffer_in_window(device_context, this);
                EndPaint(window, &paint);
            }
        }
        _ => (),
    }
    DefWindowProcA(window, message, wparam, lparam)
}

pub fn win32_process_pending_messages(window: &mut Window) {
    let mut message: MSG = Default::default();
    unsafe {
        while PeekMessageA(&mut message, HWND(0), 0, 0, PM_REMOVE).into() {
            match message.message {
                WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP => {
                    let v_k_code: char = char::from_u32(message.wParam.0 as u32)
                        .expect("Failed to parse VKCode");

                    let was_down = message.lParam.0 & (1 << 30) != 0;
                    let is_down = (message.lParam.0 & (1 << 31)) == 0;
                    let alt_key_was_down = message.lParam.0 & (1 << 29) != 0;

                    if was_down != is_down {
                        if is_down {
                            if (v_k_code as u16 == VK_F4.0) && alt_key_was_down {
                                println!("Alt+F4");
                                window.window_running = false;
                            }
                        }
                    }
                }
                _ => {
                    TranslateMessage(&message);
                    DispatchMessageA(&message);
                }
            }
        }

        win32_display_buffer_in_window(GetDC(window.handle), window);
    }
}

fn win32_display_buffer_in_window(device_context: HDC, window: &mut Window) {
    unsafe {
        let mut client_rect: RECT = Default::default();
        GetClientRect(window.handle, &mut client_rect);
        let window_width = client_rect.right - client_rect.left;
        let window_height = client_rect.bottom - client_rect.top;
        let padding = 10;

        PatBlt(
            device_context,
            0,
            0,
            window_width,
            padding,
            WHITENESS,
        );
        PatBlt(
            device_context,
            0,
            0,
            padding,
            window_height,
            WHITENESS,
        );
        PatBlt(
            device_context,
            padding + window.buffer.width,
            0,
            window_width,
            window_height,
            WHITENESS,
        );
        PatBlt(
            device_context,
            0,
            padding + window.buffer.height,
            window_width,
            window_height,
            WHITENESS,
        );

        StretchDIBits(
            device_context,
            padding,
            padding,
            window.buffer.width,
            window.buffer.height,
            0,
            0,
            window.buffer.width,
            window.buffer.height,
            Some(window.buffer.bits.as_mut_ptr() as _),
            &window.buffer.info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}
