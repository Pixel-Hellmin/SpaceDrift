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
const BACKGROUND_COLOR: Color = Color{ r: 0, g: 0, b: 0, a: 255, };

#[derive(Copy, Clone)]
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
    let mut row: usize = (pos.x as i32 * BYTES_PER_PIXEL + pos.y as i32 * buffer.width * BYTES_PER_PIXEL) as usize;
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


fn lerp(a: f32, t: f32, b: f32) -> f32 {
    // TODO(Fermin): Deal with multiple types
    (1.0 - t)*a + t*b
}

fn render_bmp(origin: &V2, x_axis: V2, y_axis: V2, bmp: &LoadedBitmap, buffer: &mut Win32OffscreenBuffer) {
    let max_x = (x_axis.x - origin.x) as i32;
    let max_y = (y_axis.y - origin.y) as i32;
    let mut dest_row: usize = (origin.x as i32 * BYTES_PER_PIXEL + origin.y as i32 * buffer.width * BYTES_PER_PIXEL) as usize;
    for y in (0..max_y).rev() {
        for x in 0..(max_x) {
            // TODO(Fermin): Check buffer bounds
            let u = x as f32 / max_x as f32;
            let v = y as f32 / max_y as f32;
            assert!(u >= 0.0 && u <= 1.0);
            assert!(v >= 0.0 && v <= 1.0);

            let texel_x = u * (bmp.width - 1) as f32;
            let texel_y = v * (bmp.height - 1) as f32;
            assert!(texel_x >= 0.0 && texel_x <= bmp.width as f32 - 1.0);
            assert!(texel_y >= 0.0 && texel_y <= bmp.height as f32 - 1.0);

            let texel_dx = texel_x - texel_x.floor();
            let texel_dy = texel_y - texel_y.floor();
            assert!(texel_dx >= 0.0 && texel_dx <= 1.0);
            assert!(texel_dy >= 0.0 && texel_dy <= 1.0);

            // NOTE(Fermin): Sub-pixel precision
            let texel_index = bmp.data_offset + texel_x as i32 * BYTES_PER_PIXEL + texel_y as i32 * bmp.pitch;
            let src_index_00 = texel_index as usize;
            let src_index_01 = (texel_index + BYTES_PER_PIXEL) as usize;
            let src_index_10 = (texel_index + bmp.pitch) as usize;
            let src_index_11 = (texel_index + bmp.pitch + BYTES_PER_PIXEL) as usize;
            assert!(src_index_00 < bmp.bits.len());
            assert!(src_index_01 < bmp.bits.len());
            assert!(src_index_10 < bmp.bits.len());
            assert!(src_index_11 < bmp.bits.len());
            
            let src_00_01_b = lerp(bmp.bits[src_index_00    ] as f32, texel_dx, bmp.bits[src_index_01    ] as f32);
            let src_00_01_g = lerp(bmp.bits[src_index_00 + 1] as f32, texel_dx, bmp.bits[src_index_01 + 1] as f32);
            let src_00_01_r = lerp(bmp.bits[src_index_00 + 2] as f32, texel_dx, bmp.bits[src_index_01 + 2] as f32);
            let src_00_01_a = lerp(bmp.bits[src_index_00 + 3] as f32, texel_dx, bmp.bits[src_index_01 + 3] as f32);

            let src_10_11_b = lerp(bmp.bits[src_index_10    ] as f32, texel_dx, bmp.bits[src_index_11    ] as f32);
            let src_10_11_g = lerp(bmp.bits[src_index_10 + 1] as f32, texel_dx, bmp.bits[src_index_11 + 1] as f32);
            let src_10_11_r = lerp(bmp.bits[src_index_10 + 2] as f32, texel_dx, bmp.bits[src_index_11 + 2] as f32);
            let src_10_11_a = lerp(bmp.bits[src_index_10 + 3] as f32, texel_dx, bmp.bits[src_index_11 + 3] as f32);

            let src_b = lerp(src_00_01_b, texel_dy, src_10_11_b);
            let src_g = lerp(src_00_01_g, texel_dy, src_10_11_g);
            let src_r = lerp(src_00_01_r, texel_dy, src_10_11_r);
            let src_a = lerp(src_00_01_a, texel_dy, src_10_11_a);

            let alpha_ratio:f32 = src_a as f32 / 255.0;

            let dest_index = dest_row + (x * BYTES_PER_PIXEL) as usize;
            let dest_b = &mut buffer.bits[dest_index];
            *dest_b = lerp(*dest_b as f32, alpha_ratio, src_b as f32) as u8; 

            let dest_g = &mut buffer.bits[dest_index + 1];
            *dest_g = lerp(*dest_g as f32, alpha_ratio, src_g as f32) as u8; 
            
            let dest_r = &mut buffer.bits[dest_index + 2];
            *dest_r = lerp(*dest_r as f32, alpha_ratio, src_r as f32) as u8; 
            
            let dest_a = &mut buffer.bits[dest_index + 3];
            *dest_a = src_a as u8; 
        }
        dest_row += (buffer.width * BYTES_PER_PIXEL) as usize;
    }
}

struct LoadedBitmap {
    bits: Vec<u8>,
    height: i32,
    width: i32,
    pitch: i32,
    data_offset: i32,
}
fn load_bitmap(file: &str) -> LoadedBitmap {
    let bits = read(file).expect("Err: Couldnt load bitmap");

    let data_offset_index = 10;
    let data_offset:i32 = 
        ((bits[data_offset_index+3] as i32) << 24) |
        ((bits[data_offset_index+2] as i32) << 16) |
        ((bits[data_offset_index+1] as i32) <<  8) |
        (bits[data_offset_index] as i32);

    let width_index = 18;
    let width:i32 =
        ((bits[width_index+3] as i32) << 24) |
        ((bits[width_index+2] as i32) << 16) |
        ((bits[width_index+1] as i32) <<  8) |
        (bits[width_index] as i32);

    let height_index = 22;
    let height:i32 = 
        ((bits[height_index+3] as i32) << 24) |
        ((bits[height_index+2] as i32) << 16) |
        ((bits[height_index+1] as i32) <<  8) |
        (bits[height_index] as i32);

    let pitch = BYTES_PER_PIXEL * width;

    LoadedBitmap { bits, height, width, pitch, data_offset }
}

struct RenderObject<'a> {
    origin: V2,
    width: i32,
    height: i32,
    bmp: &'a LoadedBitmap
}

fn update_and_render(buffer: &mut Win32OffscreenBuffer, dt_for_frame: f32, stars: &mut [Star], rng: &mut rand::rngs::ThreadRng, bmp: &LoadedBitmap) {
    // NOTE(Fermin): This solves the problem with the black rectangle
    // behind bitmaps. Look for a nicer alternative?
    let mut draw_stars: Vec<RenderObject> = Vec::new();

    for star in stars {
        // NOTE(Fermin): Erase previouse frame's stars
        draw_rectangle(&star.pos, star.width, star.height, &BACKGROUND_COLOR, buffer);

        let speed = 5.0 * star.width as f32 * dt_for_frame;
        star.pos.y += speed;

        // TODO(Fermin): When bitmap is out of bounds, cut it, dont scale it.
        if star.pos.y.round() as i32 + star.height >= buffer.height {
            star.height = star.height - (star.pos.y.round() as i32 + star.height - buffer.height);
        }

        if star.pos.y.round() as i32 >= buffer.height {
            star.pos.x = rng.gen_range(0.0..buffer.width as f32);
            star.pos.y = 0.0;
            star.width = rng.gen_range(1..20);
            star.height = star.width;
        }

        if star.pos.x.round() as i32 + star.width >= buffer.width {
            star.width = star.pos.x.round() as i32 + star.width - buffer.width;
            star.height = star.width;
        }

        draw_stars.push(RenderObject {
            origin: star.pos,
            width: star.width,
            height: star.height,
            bmp: &bmp
        });
    }

    for star in draw_stars {
        render_bmp(
            &star.origin,
            V2{x: star.origin.x + star.width as f32, y: star.origin.y},
            V2{x: star.origin.x, y: star.origin.y + star.height as f32},
            star.bmp,
            buffer
        );
    }
}

fn main() -> Result<()>{
    let mut rng:rand::rngs::ThreadRng = rand::thread_rng();

    // --------------------------------------------------------------------
    // NOTE(Fermin): Create buffer
    // --------------------------------------------------------------------
    // TODO(Fermin): Create InitBuffer routine in window.rs
    let buffer_width = 450;
    let buffer_height = 600;
    let num_of_pixels = buffer_width * buffer_height * BYTES_PER_PIXEL;
    let mut buffer = Win32OffscreenBuffer {
        info: Default::default(),
        bits: vec![0; num_of_pixels as usize], //NOTE(Fermin): Fill bg with dif color?
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
    // TODO(Fermin): Create InitWindow routine in window.rs
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
    let bmp = load_bitmap("art/star.bmp");

    // --------------------------------------------------------------------
    // NOTE(Fermin): Create collection of stars
    // --------------------------------------------------------------------
    let mut stars: Vec<Star> = Vec::new();
    for _star in 0..NUMBER_OF_STARS {
        let size = rng.gen_range(1..20);
        stars.push(Star {
            pos: V2{x: rng.gen_range(0.0..(buffer_width - size) as f32), y: rng.gen_range(0.0..(buffer_height - size) as f32)},
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
        update_and_render(&mut window.buffer, last_frame_dur / 1000.0, &mut stars, &mut rng, &bmp);

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
