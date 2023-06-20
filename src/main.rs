mod window;

use crate::window::*;
use std::fs::read;
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

fn update_and_render(buffer: &mut Win32OffscreenBuffer, dt_for_frame: f32, stars: &mut [Star], rng: &mut rand::rngs::ThreadRng) {
    let r = rng.gen_range(0..255);
    let g = rng.gen_range(0..255);
    let b = rng.gen_range(0..255);

    for star in stars {
        // NOTE(Fermin): Erase previouse frame's star
        draw_rectangle(&star.pos, star.width, star.height, &Color{ r: 0, g: 0, b: 0, a: 255, }, buffer);

        let speed = 7.0 * star.width as f32 * dt_for_frame;
        star.pos.y += speed;

        if star.pos.y.round() as i32 + star.height >= buffer.height {
            star.height = star.height - (star.pos.y.round() as i32 + star.height - buffer.height);
        }

        if star.pos.y.round() as i32 >= buffer.height {
            star.pos.x = rng.gen_range(0.0..buffer.width as f32);
            star.pos.y = 0.0;
            star.width = rng.gen_range(1..20);
            star.height = star.width;
        }

        draw_rectangle(&star.pos, star.width, star.height, &Color{ r, g, b, a: 255, }, buffer);
    }

}

fn lerp(a: f32, t: f32, b: f32) -> f32 {
    // TODO(Fermin): Deal with multiple types
    (1.0 - t)*a + t*b
}

fn render_bmp(origin: V2, x_axis: V2, y_axis: V2, bmp: &Vec<u8>, buffer: &mut Win32OffscreenBuffer) {
    let bmp_data_offset_index = 10;
    let bmp_data_offset:i32 = ((bmp[bmp_data_offset_index+3] as i32) << 24) | ((bmp[bmp_data_offset_index+2] as i32) << 16) | ((bmp[bmp_data_offset_index+1] as i32) << 8) | (bmp[bmp_data_offset_index] as i32);

    let bmp_width_index = 18;
    let bmp_width:i32 = ((bmp[bmp_width_index+3] as i32) << 24) | ((bmp[bmp_width_index+2] as i32) << 16) | ((bmp[bmp_width_index+1] as i32) << 8) | (bmp[bmp_width_index] as i32);

    let bmp_height_index = 22;
    let bmp_height:i32 = ((bmp[bmp_height_index+3] as i32) << 24) | ((bmp[bmp_height_index+2] as i32) << 16) | ((bmp[bmp_height_index+1] as i32) << 8) | (bmp[bmp_height_index] as i32);

    let max_x = (x_axis.x - origin.x) as i32;
    let max_y = (y_axis.y - origin.y) as i32;
    let mut dest_row: usize = (origin.x as i32 * BYTES_PER_PIXEL + origin.y as i32 * buffer.width * BYTES_PER_PIXEL) as usize;
    for y in (0..max_y).rev() {
        for x in 0..(max_x) {
            let u = x as f32 / max_x as f32;
            let v = y as f32 / max_y as f32;

            assert!(u >= 0.0 && u <= 1.0);
            assert!(v >= 0.0 && v <= 1.0);

            let u_src = (u * bmp_width as f32).round() as i32;
            let v_src = (v * bmp_height as f32).round() as i32;

            let src_index = bmp_data_offset + u_src * BYTES_PER_PIXEL + v_src * bmp_width * BYTES_PER_PIXEL;
            let src_b = bmp[(src_index) as usize];
            let src_g = bmp[(src_index + 1) as usize];
            let src_r = bmp[(src_index + 2) as usize];
            let src_a = bmp[(src_index + 3) as usize];

            let alpha_ratio:f32 = src_a as f32 / 255.0;

            let dest_index = dest_row + (x * BYTES_PER_PIXEL) as usize;
            let dest_b = &mut buffer.bits[dest_index];
            *dest_b = lerp(*dest_b as f32, alpha_ratio, src_b as f32) as u8; 

            let dest_g = &mut buffer.bits[dest_index + 1];
            *dest_g = lerp(*dest_g as f32, alpha_ratio, src_g as f32) as u8; 
            
            let dest_r = &mut buffer.bits[dest_index + 2];
            *dest_r = lerp(*dest_r as f32, alpha_ratio, src_r as f32) as u8; 
            
            let dest_a = &mut buffer.bits[dest_index + 3];
            *dest_a = src_a; 
        }
        dest_row += (buffer.width * BYTES_PER_PIXEL) as usize;
    }
}

fn main() -> Result<()>{
    let mut rng:rand::rngs::ThreadRng = rand::thread_rng();

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
    // NOTE(Fermin): Load test bitmap
    // --------------------------------------------------------------------
    // NOTE(Fermin): Loading this bitmap tanks the fps
    let bmp = read("art/two_dots_astro.bmp").expect("Err: Couldnt load bitmap");

    // --------------------------------------------------------------------
    // NOTE(Fermin): Create collection of stars
    // --------------------------------------------------------------------
    let mut stars: Vec<Star> = Vec::new();
    for _star in 0..NUMBER_OF_STARS {
        let size = rng.gen_range(1..20);
        stars.push(Star {
            pos: V2{x: rng.gen_range(0.0..buffer_width as f32), y: rng.gen_range(0.0..buffer_height as f32)},
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
        update_and_render(&mut window.buffer, last_frame_dur / 1000.0, &mut stars, &mut rng);
        render_bmp(
            V2{x: 10.0, y: 10.0},
            V2{x: 300.0, y: 10.0},
            V2{x: 10.0, y: 500.0},
            &bmp,
            &mut window.buffer
        );

        // --------------------------------------------------------------------
        // NOTE(Fermin): Sleep thread if necessary to sync with monitor refresh rate
        // --------------------------------------------------------------------
        let target_ms_per_frame = (target_seconds_per_frame * 1000.0) as u128;
        let time_elapsed_since_frame_start = frame_start_instant.elapsed().as_millis();
        if time_elapsed_since_frame_start < target_ms_per_frame {
            let ms_until_next_frame: u64 = (target_ms_per_frame - time_elapsed_since_frame_start)
            .try_into()
            .expect("Error calculating ms until next frame");
            std::thread::sleep(Duration::from_millis(ms_until_next_frame));
        }
        last_frame_dur = frame_start_instant.elapsed().as_millis() as f32;
        println!("{} fps, {} ms/f", 1.0/last_frame_dur*1000.0, last_frame_dur);
    }

    Ok(())
}
