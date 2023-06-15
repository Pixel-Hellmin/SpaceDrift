mod window;

use crate::window::*;
use std::time::{Duration, Instant};
use rand::Rng;
use windows::{
    core::Result,
    s,
    Win32::{
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::*,
        Foundation::{HINSTANCE, HWND},
        Media::timeBeginPeriod
    }
};

const BYTES_PER_PIXEL: i32 = 4;
const NUMBER_OF_STARS: i32 = 40;

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

struct Star {
    pos: V2,
    width: i32,
    height: i32,
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
    
    // TODO(Fermin): Allow for partial stars to be drawn in the horizontal axis
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

fn update_and_render(buffer: &mut Win32OffscreenBuffer, dt_for_frame: f32, stars: &mut [Star]) {
    let r = rand::thread_rng().gen_range(0..255);
    let g = rand::thread_rng().gen_range(0..255);
    let b = rand::thread_rng().gen_range(0..255);

    for star in stars {
        // NOTE(Fermin): Erase previouse frame's star
        draw_rectangle(&star.pos, star.width, star.height, &Color{ r: 0, g: 0, b: 0, a: 255, }, buffer);

        let speed = 7.0 * star.width as f32 * dt_for_frame;
        star.pos.y += speed;

        if star.pos.y.round() as i32 + star.height >= buffer.height {
            star.height = star.height - (star.pos.y.round() as i32 + star.height - buffer.height);
        }

        if star.pos.y.round() as i32 >= buffer.height {
            star.pos.x = rand::thread_rng().gen_range(0.0..buffer.width as f32);
            star.pos.y = 0.0;
            star.width = rand::thread_rng().gen_range(1..20);
            star.height = star.width;
        }

        draw_rectangle(&star.pos, star.width, star.height, &Color{ r, g, b, a: 255, }, buffer);
    }

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
    // NOTE(Fermin): Create collection of stars
    // --------------------------------------------------------------------
    let mut stars: Vec<Star> = Vec::new();
    for _star in 0..NUMBER_OF_STARS {
        let size = rand::thread_rng().gen_range(1..20);
        stars.push(Star {
            pos: V2{x: rand::thread_rng().gen_range(0.0..buffer_width as f32), y: rand::thread_rng().gen_range(0.0..buffer_height as f32)},
            width: size,
            height: size,
        })
    }

    // --------------------------------------------------------------------
    // NOTE(Fermin): Main loop
    // --------------------------------------------------------------------
    let target_seconds_per_frame: f32 = 1.0 / window.refresh_rate as f32;
    let mut last_frame_dur = target_seconds_per_frame;

    while window.window_running {
        let frame_start_instant = Instant::now();

        win32_process_pending_messages(window.as_mut());
        update_and_render(&mut window.buffer, last_frame_dur / 1000.0, &mut stars);

        let target_ms_per_frame = (target_seconds_per_frame * 1000.0) as u128;
        let time_elapsed_since_frame_start = frame_start_instant.elapsed().as_millis();
        if time_elapsed_since_frame_start < target_ms_per_frame {
            let ms_until_next_frame: u64 = (target_ms_per_frame - time_elapsed_since_frame_start)
            .try_into()
            .expect("Error calculating ms until next frame");
            std::thread::sleep(Duration::from_millis(ms_until_next_frame));
        }
        last_frame_dur = frame_start_instant.elapsed().as_millis() as f32;
        println!("{} ms/f", last_frame_dur);
    }

    Ok(())
}
