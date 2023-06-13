use windows::{
    core::{Result, Error, PCSTR},
    s,
    Win32::{
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::*,
        Foundation::{RECT, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Media::timeBeginPeriod
    }
};

const WINDOW_CLASS_NAME: PCSTR = s!("win32.Window");

pub struct Win32OffscreenBuffer {
    // Pixels always are 32-bits wide, Memory Order BB GG RR XX
    info: BITMAPINFO,
    pub bits: Vec<i32>,
    pub width: i32,
    pub height: i32,
}

pub struct Window {
    handle: HWND,
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

unsafe extern "system" fn win32_main_window_callback(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => {
            println!("CREATE");

            let cs = lparam.0 as *const CREATESTRUCTW;
            let this = (*cs).lpCreateParams as *mut Window;
            (*this).handle = window;

            SetWindowLongPtrA(window, GWLP_USERDATA, this as _);
        }
        WM_CLOSE | WM_DESTROY => {
            println!("WM_CLOSE|WN_DESTROY");

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
            println!("WM_PAINT");

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
                WM_MOUSEMOVE => {
                }
                // NOTE(Fermin): Consider following the same logic for
                // mouse button than keyboard buttons
                WM_LBUTTONDOWN => {
                    println!("WM_LBUTTONDOWN");
                }
                WM_LBUTTONUP => {
                    println!("WM_LBUTTONUP");
                }
                WM_RBUTTONDOWN => {
                    println!("WM_RBUTTONDOWN");
                }
                WM_RBUTTONUP => {
                    println!("WM_RBUTTONUP");
                }
                WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP => {
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
        //let device_context = ;
        let window_width = client_rect.right - client_rect.left;
        let window_height = client_rect.bottom - client_rect.top;

        PatBlt(
            device_context,
            0,
            0,
            window_width,
            10,
            BLACKNESS,
        );
        PatBlt(
            device_context,
            0,
            0,
            10,
            window_height,
            BLACKNESS,
        );
        PatBlt(
            device_context,
            10 + window.buffer.width,
            0,
            window_width,
            window_height,
            BLACKNESS,
        );
        PatBlt(
            device_context,
            0,
            10 + window.buffer.height,
            window_width,
            window_height,
            BLACKNESS,
        );

        StretchDIBits(
            device_context,
            10,
            10,
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

fn main() -> Result<()>{
    // --------------------------------------------------------------------
    // NOTE(Fermin): Create buffer
    // --------------------------------------------------------------------
    let buffer_width = 500;
    let buffer_height = 500;
    //let bytes_per_pixel: i32 = 4;
    //let bitmap_memory_size: usize = ((buffer_width * buffer_height) * bytes_per_pixel) as usize;
    let mut buffer = Win32OffscreenBuffer {
        info: Default::default(),
        bits: Vec::new(),
        width: buffer_width,
        height: buffer_height,
    };
    buffer.info.bmiHeader.biWidth = buffer_width;
    buffer.info.bmiHeader.biHeight = -buffer_height; // - sign so origin is top left
    buffer.info.bmiHeader.biPlanes = 1;
    buffer.info.bmiHeader.biBitCount = 32; // 3 bytes for RGB (one each) and one byte for padding cus it needs to be aligned in blocks of 4 bytes
    buffer.info.bmiHeader.biCompression = BI_RGB;
    buffer.info.bmiHeader.biSize = (std::mem::size_of::<BITMAPINFOHEADER>())
        .try_into()
        .expect("Error computing BITMAPINFOHEADER size");

    // --------------------------------------------------------------------
    // NOTE(Fermin): Create window
    // --------------------------------------------------------------------
    let instance = unsafe { GetModuleHandleA(None)? };
    let class = WNDCLASSA {
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
        hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
        hInstance: instance,
        lpszClassName: WINDOW_CLASS_NAME,
        lpfnWndProc: Some(win32_main_window_callback),
        ..Default::default()
    };
    assert_ne!(unsafe { RegisterClassA(&class) }, 0);

    let mut window = Box::new(Window {
        handle: HWND(0),
        buffer,
        window_running: true,
        refresh_rate: 60,
    });

    let window_tmp = unsafe {
        CreateWindowExA(
            WS_EX_LEFT, // ms: WS_EX_NOREDIRECTIONBITMAP, hmh: 0
            WINDOW_CLASS_NAME,
            &s!("Space Drift"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            buffer_width + 20,
            buffer_height + 20,
            HWND(0),
            HMENU(0),
            instance,
            Some(window.as_mut() as *mut _ as _),
        )
        .ok()? //NOTE(Fermin): Consider removing this trait
    };
    unsafe { timeBeginPeriod(1); }
    window.refresh_rate = unsafe { GetDeviceCaps(GetDC(window_tmp), VREFRESH) };

    while window.window_running {
        win32_process_pending_messages(window.as_mut());
    }

    Ok(())
}
