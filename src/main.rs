use windows::{
    core::{Result, Error, PCSTR},
    s,
    Win32::{
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::*,
        UI::Input::KeyboardAndMouse::VK_F4,
        Foundation::{RECT, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Media::timeBeginPeriod
    }
};

const WINDOW_CLASS_NAME: PCSTR = s!("win32.Window");
const BYTES_PER_PIXEL: i32 = 4;

struct Win32OffscreenBuffer {
    // Pixels always are 32-bits wide, Memory Order BB GG RR XX
    info: BITMAPINFO,
    pub bits: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

struct Window {
    handle: HWND,
    buffer: Win32OffscreenBuffer,
    window_running: bool,
    refresh_rate: i32,
}
trait CheckHandle: Sized {
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

struct V2 {
    x: f32,
    y: f32
}

struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

unsafe extern "system" fn win32_main_window_callback(
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

fn win32_process_pending_messages(window: &mut Window) {
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

fn draw_rectangle(
    pos: &V2,
    width: i32,
    height: i32,
    color: &Color,
    buffer: &mut Win32OffscreenBuffer,
) {
    let start_x: i32;
    let start_y: i32;
    
    if pos.x + width as f32 > buffer.width as f32 {
        start_x = buffer.width - width;
    } else if pos.x < 0.0 {
        start_x = 0;
    } else {
        start_x = pos.x.round() as i32;
    }

    if pos.y + height as f32 > buffer.height as f32 {
        start_y = buffer.height - height;
    } else if pos.y < 0.0 {
        start_y = 0;
    } else {
        start_y = pos.y.round() as i32;
    }

    let mut row: usize = (start_x * BYTES_PER_PIXEL + start_y * buffer.width * BYTES_PER_PIXEL) as usize;
    for _y in 0..height {
        for x in 0..width {
            // NOTE(Fermin): Pixel -> BB GG RR AA
            buffer.bits[row + (x * BYTES_PER_PIXEL) as usize] = color.b;
            buffer.bits[row + (x * BYTES_PER_PIXEL + 1) as usize] = color.g;
            buffer.bits[row + (x * BYTES_PER_PIXEL + 2) as usize] = color.r;
            buffer.bits[row + (x * BYTES_PER_PIXEL + 3) as usize] = color.a;
        }
        row += (buffer.width * BYTES_PER_PIXEL) as usize;
    }
}

fn update_and_render(buffer: &mut Win32OffscreenBuffer) {
    draw_rectangle(&V2{x:200.0, y:200.0}, 15, 15, &Color{ r: 250, g: 193, b: 235, a: 255, }, buffer);
}

fn main() -> Result<()>{
    // --------------------------------------------------------------------
    // NOTE(Fermin): Create buffer
    // --------------------------------------------------------------------
    let buffer_width = 450;
    let buffer_height = 600;
    let num_of_pixels = buffer_width * buffer_height * BYTES_PER_PIXEL;
    let mut buffer = Win32OffscreenBuffer {
        info: Default::default(),
        bits: vec![0; num_of_pixels as usize],
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

    unsafe {
        let window_tmp = CreateWindowExA(
            WS_EX_LEFT, // ms: WS_EX_NOREDIRECTIONBITMAP, hmh: 0
            WINDOW_CLASS_NAME,
            &s!("Space Drift"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            buffer_width + 40,
            buffer_height + 60,
            HWND(0),
            HMENU(0),
            instance,
            Some(window.as_mut() as *mut _ as _),
        )
        .ok()?;
        timeBeginPeriod(1);
        window.refresh_rate = GetDeviceCaps(GetDC(window_tmp), VREFRESH);
    }

    // --------------------------------------------------------------------
    // NOTE(Fermin): Main loop
    // --------------------------------------------------------------------
    while window.window_running {
        win32_process_pending_messages(window.as_mut());
        update_and_render(&mut window.buffer);
    }

    Ok(())
}
