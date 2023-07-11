#![windows_subsystem = "windows"]

mod window;

use crate::window::*;
use rand::Rng;
//use std::fs::read;
use std::time::{Duration, Instant};
use windows::{ core::Result, s };

pub const BYTES_PER_PIXEL: i32 = 4;
const NUMBER_OF_STARS: i32 = 60;
const BACKGROUND_COLOR: Color = Color {
    r: 64,
    g: 18,
    b: 139,
    a: 255,
};
const STAR_COLOR: Color = Color {
    r: 249,
    g: 217,
    b: 73,
    a: 255,
};
const MAX_STAR_RADIUS: i32 = 12;
const MIN_STAR_RADIUS: i32 = 2;

#[derive(Copy, Clone)]
struct V2 {
    x: f32,
    y: f32,
}
impl std::ops::Add<V2> for V2 {
    type Output = V2;

    fn add(self, a: V2) -> V2 {
        V2 {
            x: self.x + a.x,
            y: self.y + a.y,
        }
    }
}
impl std::ops::Sub<V2> for V2 {
    type Output = V2;

    fn sub(self, a: V2) -> V2 {
        V2 {
            x: self.x - a.x,
            y: self.y - a.y,
        }
    }
}
fn v2_length(a: V2) -> f32 {
    (a.x * a.x + a.y * a.y).sqrt()
}

struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

struct Star {
    origin: V2,
    radius: i32,
}

fn draw_rectangle(
    pos: &V2,
    width: i32,
    height: i32,
    color: &Color,
    buffer: &mut Win32OffscreenBuffer,
) {
    let row_x_index = pos.x.clamp(0.0, buffer.width as f32 - 1.0) as i32;
    let row_y_index = pos.y.clamp(0.0, buffer.height as f32 - 1.0) as i32;
    let mut row: usize = (row_x_index * BYTES_PER_PIXEL + row_y_index * buffer.pitch) as usize;

    for y in 0..height {
        let mut drawn = false;
        let mut dest_index = row;
        for x in 0..width {
            let pixel_x = x + pos.x as i32;
            let pixel_y = y + pos.y as i32;
            if pixel_y >= 0 && pixel_y < buffer.height && pixel_x >= 0 && pixel_x < buffer.width {
                // NOTE(Fermin): Pixel -> BB GG RR AA
                buffer.bits[dest_index] = color.b;
                buffer.bits[dest_index + 1] = color.g;
                buffer.bits[dest_index + 2] = color.r;
                buffer.bits[dest_index + 3] = color.a;

                drawn = true;
                dest_index += BYTES_PER_PIXEL as usize;
            }
        }
        if drawn {
            row += (buffer.pitch) as usize;
        }
    }
}

fn lerp(a: f32, t: f32, b: f32) -> f32 {
    // TODO(Fermin): Deal with multiple types
    (1.0 - t) * a + t * b
}

/*
fn render_bmp(
    origin: &V2,
    x_axis: V2,
    y_axis: V2,
    bmp: &LoadedBitmap,
    buffer: &mut Win32OffscreenBuffer,
) {
    let max_width = buffer.width - 1;
    let max_height = buffer.height - 1;
    let mut x_min = max_width;
    let mut x_max = 0;
    let mut y_min = max_height;
    let mut y_max = 0;

    let bmp_corners = [*origin, x_axis, x_axis + y_axis - *origin, y_axis];
    for corner in bmp_corners {
        let floor_x = corner.x.floor() as i32;
        let ceil_x = corner.x.ceil() as i32;
        let floor_y = corner.y.floor() as i32;
        let ceil_y = corner.y.ceil() as i32;

        if x_min > floor_x {
            x_min = floor_x;
        }
        if x_max < ceil_x {
            x_max = ceil_x;
        }
        if y_min > floor_y {
            y_min = floor_y;
        }
        if y_max < ceil_y {
            y_max = ceil_y;
        }
    }

    let mut dest_row: usize =
        (x_min as i32 * BYTES_PER_PIXEL + y_min as i32 * buffer.pitch) as usize;
    for y in y_min..=y_max {
        let mut dest_index = dest_row;
        let mut drawn = false;
        for x in x_min..=x_max {
            if y >= 0 && y < max_height && x >= 0 && x < max_width {
                let u = (x_max - x) as f32 / (x_max - x_min) as f32;
                let v = (y_max - y) as f32 / (y_max - y_min) as f32;
                assert!(u >= 0.0 && u <= 1.0);
                assert!(v >= 0.0 && v <= 1.0);

                let texel_x = u * (bmp.width - 2) as f32;
                let texel_y = v * (bmp.height - 2) as f32;

                let texel_dx = texel_x - texel_x.floor();
                let texel_dy = texel_y - texel_y.floor();

                // NOTE(Fermin): Sub-pixel precision
                let texel_index =
                    bmp.data_offset + texel_x as i32 * BYTES_PER_PIXEL + texel_y as i32 * bmp.pitch;
                let src_index_00 = texel_index as usize;
                let src_index_01 = (texel_index + BYTES_PER_PIXEL) as usize;
                let src_index_10 = (texel_index + bmp.pitch) as usize;
                let src_index_11 = (texel_index + bmp.pitch + BYTES_PER_PIXEL) as usize;

                let src_00_01_b = lerp(
                    bmp.bits[src_index_00] as f32,
                    texel_dx,
                    bmp.bits[src_index_01] as f32,
                );
                let src_00_01_g = lerp(
                    bmp.bits[src_index_00 + 1] as f32,
                    texel_dx,
                    bmp.bits[src_index_01 + 1] as f32,
                );
                let src_00_01_r = lerp(
                    bmp.bits[src_index_00 + 2] as f32,
                    texel_dx,
                    bmp.bits[src_index_01 + 2] as f32,
                );
                let src_00_01_a = lerp(
                    bmp.bits[src_index_00 + 3] as f32,
                    texel_dx,
                    bmp.bits[src_index_01 + 3] as f32,
                );

                let src_10_11_b = lerp(
                    bmp.bits[src_index_10] as f32,
                    texel_dx,
                    bmp.bits[src_index_11] as f32,
                );
                let src_10_11_g = lerp(
                    bmp.bits[src_index_10 + 1] as f32,
                    texel_dx,
                    bmp.bits[src_index_11 + 1] as f32,
                );
                let src_10_11_r = lerp(
                    bmp.bits[src_index_10 + 2] as f32,
                    texel_dx,
                    bmp.bits[src_index_11 + 2] as f32,
                );
                let src_10_11_a = lerp(
                    bmp.bits[src_index_10 + 3] as f32,
                    texel_dx,
                    bmp.bits[src_index_11 + 3] as f32,
                );

                let src_b = lerp(src_00_01_b, texel_dy, src_10_11_b);
                let src_g = lerp(src_00_01_g, texel_dy, src_10_11_g);
                let src_r = lerp(src_00_01_r, texel_dy, src_10_11_r);
                let src_a = lerp(src_00_01_a, texel_dy, src_10_11_a);

                let alpha_ratio: f32 = src_a as f32 / 255.0;

                let dest_b = &mut buffer.bits[dest_index];
                *dest_b = lerp(*dest_b as f32, alpha_ratio, src_b as f32) as u8;

                let dest_g = &mut buffer.bits[dest_index + 1];
                *dest_g = lerp(*dest_g as f32, alpha_ratio, src_g as f32) as u8;

                let dest_r = &mut buffer.bits[dest_index + 2];
                *dest_r = lerp(*dest_r as f32, alpha_ratio, src_r as f32) as u8;

                let dest_a = &mut buffer.bits[dest_index + 3];
                *dest_a = src_a as u8;

                dest_index += BYTES_PER_PIXEL as usize;
                drawn = true;
            }
        }
        if drawn {
            dest_row += (buffer.pitch) as usize;
        }
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
    let data_offset: i32 = ((bits[data_offset_index + 3] as i32) << 24)
        | ((bits[data_offset_index + 2] as i32) << 16)
        | ((bits[data_offset_index + 1] as i32) << 8)
        | (bits[data_offset_index] as i32);

    let width_index = 18;
    let width: i32 = ((bits[width_index + 3] as i32) << 24)
        | ((bits[width_index + 2] as i32) << 16)
        | ((bits[width_index + 1] as i32) << 8)
        | (bits[width_index] as i32);

    let height_index = 22;
    let height: i32 = ((bits[height_index + 3] as i32) << 24)
        | ((bits[height_index + 2] as i32) << 16)
        | ((bits[height_index + 1] as i32) << 8)
        | (bits[height_index] as i32);

    let pitch = BYTES_PER_PIXEL * width;

    LoadedBitmap {
        bits,
        height,
        width,
        pitch,
        data_offset,
    }
}
*/

fn draw_star(star: &Star, buffer: &mut Win32OffscreenBuffer) {
    let top_left = star.origin
        - V2 {
            x: star.radius as f32,
            y: star.radius as f32,
        };
    let row_x_index = top_left.x.clamp(0.0, buffer.width as f32 - 1.0) as i32;
    let row_y_index = top_left.y.clamp(0.0, buffer.height as f32 - 1.0) as i32;
    let mut row: usize = (row_x_index * BYTES_PER_PIXEL + row_y_index * buffer.pitch) as usize;

    for y in 0..star.radius * 2 {
        let mut drawn = false;
        let mut dest_index: usize = row;
        for x in 0..star.radius * 2 {
            let pixel_x = top_left.x as i32 + x;
            let pixel_y = top_left.y as i32 + y;
            if pixel_y >= 0 && pixel_y < buffer.height && pixel_x >= 0 && pixel_x < buffer.width {
                let pixel_radius = v2_length( V2 { x: pixel_x as f32, y: pixel_y as f32, } - star.origin);
                // NOTE(Fermin): We need to clamp because we are iterating on a square,
                // so some pixels(corners) will be further away than radius of the star
                // resulting in a negative opacity
                let mut pixel_opacity = (1.0 - (pixel_radius / star.radius as f32)).clamp(0.0, 1.0);
                if pixel_opacity >= 0.85 {
                    pixel_opacity = 1.0;
                }

                // TODO(Fermin): Define star color
                let src_b = STAR_COLOR.b as f32;
                let src_g = STAR_COLOR.g as f32 ;
                let src_r = STAR_COLOR.r as f32;
                let src_a = (STAR_COLOR.a as f32 * pixel_opacity).round();
                let color_t = src_a / 255.0;

                buffer.bits[dest_index] =
                    lerp(buffer.bits[dest_index] as f32, color_t, src_b) as u8;
                buffer.bits[dest_index + 1] =
                    lerp(buffer.bits[dest_index + 1] as f32, color_t, src_g) as u8;
                buffer.bits[dest_index + 2] =
                    lerp(buffer.bits[dest_index + 2] as f32, color_t, src_r) as u8;
                buffer.bits[dest_index + 3] = src_a as u8;

                drawn = true;
                dest_index += BYTES_PER_PIXEL as usize;
            }
        }
        if drawn {
            row += (buffer.pitch) as usize;
        }
    }
}

fn update_and_render(
    buffer: &mut Win32OffscreenBuffer,
    dt_for_frame: f32,
    stars: &mut [Star],
    rng: &mut rand::rngs::ThreadRng,
) {
    for star in &mut *stars {
        // NOTE(Fermin): Erase previouse frame's star
        draw_rectangle(
            &(star.origin
                - V2 {
                    x: star.radius as f32,
                    y: star.radius as f32,
                }),
            star.radius * 2,
            star.radius * 2,
            &BACKGROUND_COLOR,
            buffer,
        );

        let speed = 4.0 * star.radius as f32 * dt_for_frame;
        star.origin.y += speed;

        let half_radius = (star.radius / 2) as f32;
        if star.origin.y.round() as i32 - star.radius >= buffer.height {
            star.radius = rng.gen_range(MIN_STAR_RADIUS..MAX_STAR_RADIUS);
            star.origin.x = rng.gen_range(-half_radius..buffer.width as f32 -half_radius);
            star.origin.y = -star.radius as f32;
        }
    }

    // NOTE(Fermin): We erase in the first loop and draw in this one to avoid
    // erasing stars that overlap
    for star in stars {
        /*
        render_bmp(
            &star.pos,
            V2{x: star.pos.x + star.width as f32, y: star.pos.y},
            V2{x: star.pos.x, y: star.pos.y + star.height as f32},
            bmp,
            buffer
        );
        */
        draw_star(&star, buffer);
    }
}

fn main() -> Result<()> {
    // TODO(Fermin): Make buffer the same size as the window instead of 
    // fixed values.
    let mut window = get_window(1920, 1080, &s!("Space Drift"))
        .ok()
        .expect("Err: at fn call init_window");
    
    // --------------------------------------------------------------------
    // NOTE(Fermin): Fill buffer with background color
    // --------------------------------------------------------------------
    let mut dest_index: usize = 0;
    for _y in 0..window.buffer.height {
        for _x in 0..window.buffer.width {
            // NOTE(Fermin): Pixel -> BB GG RR AA
            window.buffer.bits[dest_index] = BACKGROUND_COLOR.b;
            window.buffer.bits[dest_index + 1] = BACKGROUND_COLOR.g;
            window.buffer.bits[dest_index + 2] = BACKGROUND_COLOR.r;
            window.buffer.bits[dest_index + 3] = BACKGROUND_COLOR.a;

            dest_index += BYTES_PER_PIXEL as usize;
        }
    }

    // --------------------------------------------------------------------
    // NOTE(Fermin): Load test bitmap. This bitmap is not used since we draw 
    // stars now. I'll leave it for now in case we load something later.
    // --------------------------------------------------------------------
    //let bmp = load_bitmap("art/star.bmp");

    // --------------------------------------------------------------------
    // NOTE(Fermin): Create collection of stars
    // --------------------------------------------------------------------
    let mut rng: rand::rngs::ThreadRng = rand::thread_rng();

    let mut stars: Vec<Star> = Vec::new();
    for _star in 0..NUMBER_OF_STARS {
        let radius = rng.gen_range(MIN_STAR_RADIUS..MAX_STAR_RADIUS);
        let half_radius = (radius / 2) as f32;
        stars.push(Star {
            origin: V2 {
                x: rng.gen_range(-half_radius..window.buffer.width as f32 - half_radius),
                y: rng.gen_range(-radius as f32..(window.buffer.height - radius) as f32),
            },
            radius,
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
        update_and_render(
            &mut window.buffer,
            last_frame_dur / 1000.0,
            &mut stars,
            &mut rng,
        );

        // --------------------------------------------------------------------
        // NOTE(Fermin): Sleep thread if necessary to sync with monitor refresh rate.
        // Should this be in Window?
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
        println!(
            "{} fps, {} ms/f",
            1.0 / last_frame_dur * 1000.0,
            last_frame_dur
        );
    }

    Ok(())
}
