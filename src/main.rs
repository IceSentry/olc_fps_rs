#![feature(slice_fill)]

use std::{cmp::Ordering, ptr, time::Instant};
use winapi::{
    shared::ntdef::NULL,
    um::{
        wincon::{
            CreateConsoleScreenBuffer, SetConsoleActiveScreenBuffer, WriteConsoleOutputCharacterW,
            CONSOLE_TEXTMODE_BUFFER,
        },
        wincontypes::COORD,
        winnt::{GENERIC_READ, GENERIC_WRITE, HANDLE},
        winuser::GetAsyncKeyState,
    },
};

const SCREEN_WIDTH: usize = 120;
const SCREEN_HEIGHT: usize = 40;
const SCREEN_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

const MAP_HEIGHT: usize = 16;
const MAP_WIDTH: usize = 16;

const FOV: f32 = std::f32::consts::PI / 4.0;

const DEPTH: f32 = 16.0;

struct Player {
    x: f32,
    y: f32,
    a: f32,
}

#[cfg(windows)]
fn main() {
    let mut player = Player {
        x: 8.0,
        y: 8.0,
        a: 0.0,
    };

    let mut screen: Vec<u16> = init_screen();
    let h_console = create_console_buffer();
    let mut bytes_written: u32 = 0;

    let map = init_map();

    let mut start;
    let mut end = Instant::now();

    // Game loop
    loop {
        start = Instant::now();
        let delta_time = start - end;
        end = start;
        let delta_time = delta_time.as_secs_f32();

        handle_controls(&mut player, delta_time, &map);
        update_screen(&mut screen, &player, &map);

        let stats = format!(
            "X={}, Y={}, A={}, FPS={}",
            player.x,
            player.y,
            player.a,
            1.0 / delta_time
        );

        for (i, c) in stats.chars().enumerate() {
            screen[i] = c as u16;
        }

        draw_map(&mut screen, &player, &map);
        draw_screen_to_console(h_console, &mut screen, &mut bytes_written);
    }
}

fn create_console_buffer() -> HANDLE {
    let h_console;
    unsafe {
        h_console = CreateConsoleScreenBuffer(
            GENERIC_READ | GENERIC_WRITE,
            0,
            ptr::null(),
            CONSOLE_TEXTMODE_BUFFER,
            NULL,
        );
        SetConsoleActiveScreenBuffer(h_console);
    }
    h_console
}

fn init_screen() -> Vec<u16> {
    let mut screen = Vec::with_capacity(SCREEN_SIZE);
    for _ in 0..=SCREEN_SIZE {
        screen.push(0);
    }
    screen
}

fn init_map() -> Vec<char> {
    let mut map = String::new();
    map.push_str("################");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("#..........#...#");
    map.push_str("#..........#...#");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("#.......########");
    map.push_str("#..............#");
    map.push_str("#..............#");
    map.push_str("################");
    map.chars().collect()
}

fn is_wall(map: &[char], x: usize, y: usize) -> bool {
    map[y * MAP_WIDTH + x] == '#'
}

fn draw_map(screen: &mut [u16], player: &Player, map: &[char]) {
    for nx in 0..MAP_WIDTH {
        for ny in 0..MAP_HEIGHT {
            screen[(ny + 1) * SCREEN_WIDTH + nx] =
                if player.y as usize == ny && player.x as usize == nx {
                    'P' as u16
                } else {
                    map[ny * MAP_WIDTH + nx] as u16
                };
        }
    }
}

fn handle_controls(player: &mut Player, delta_time: f32, map: &[char]) {
    let rotation_speed = 0.75;
    let move_speed = 5.0;
    unsafe {
        if GetAsyncKeyState('A' as i32) != 0 {
            player.a -= move_speed * rotation_speed * delta_time;
        }
        if GetAsyncKeyState('D' as i32) != 0 {
            player.a += move_speed * rotation_speed * delta_time;
        }
        if GetAsyncKeyState('W' as i32) != 0 {
            let x_offset = player.a.sin() * move_speed * delta_time;
            let y_offset = player.a.cos() * move_speed * delta_time;
            player.x += x_offset;
            player.y += y_offset;
            if is_wall(map, player.x as usize, player.y as usize) {
                player.x -= x_offset;
                player.y -= y_offset;
            }
        }
        if GetAsyncKeyState('S' as i32) != 0 {
            let x_offset = player.a.sin() * move_speed * delta_time;
            let y_offset = player.a.cos() * move_speed * delta_time;
            player.x -= x_offset;
            player.y -= y_offset;
            if is_wall(map, player.x as usize, player.y as usize) {
                player.x += x_offset;
                player.y += y_offset;
            }
        }
    }
}

fn update_screen(screen: &mut [u16], player: &Player, map: &[char]) {
    for x in 0..SCREEN_WIDTH {
        let ray_angle = (player.a - FOV / 2.0) + (x as f32 / SCREEN_WIDTH as f32) * FOV;
        let mut distance_to_wall = 0.0;
        let mut boundary = false;

        let eye_x = ray_angle.sin();
        let eye_y = ray_angle.cos();
        loop {
            distance_to_wall += 0.1;

            let test_x = (player.x + eye_x * distance_to_wall) as i32;
            let test_y = (player.y + eye_y * distance_to_wall) as i32;

            if test_x < 0 || test_x >= MAP_WIDTH as i32 || test_y < 0 || test_y >= MAP_HEIGHT as i32
            {
                distance_to_wall = DEPTH;
                break;
            } else if is_wall(map, test_x as usize, test_y as usize) {
                let mut p: Vec<(f32, f32)> = Vec::new();
                for tx in 0..2 {
                    for ty in 0..2 {
                        let vy = test_y as f32 + ty as f32 - player.y;
                        let vx = test_x as f32 + tx as f32 - player.x;
                        let d = (vx * vx + vy * vy).sqrt();
                        let dot = (eye_x * vx / d) + (eye_y * vy / d);
                        p.push((d, dot));
                    }
                }

                p.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or_else(|| Ordering::Equal));

                let bound = 0.01;
                boundary = p[0].1.acos() < bound || p[1].1.acos() < bound;
                break;
            }
        }

        let ceiling = (SCREEN_HEIGHT as f32 / 2.0 - SCREEN_HEIGHT as f32 / distance_to_wall) as i32;
        let floor = SCREEN_HEIGHT as i32 - ceiling;

        for y in 0..SCREEN_HEIGHT {
            let y = y as i32;
            let x = x as i32;
            let index = y * SCREEN_WIDTH as i32 + x;

            screen[index as usize] = if y < ceiling {
                ' ' as u16 // ceiling
            } else if y > ceiling && y <= floor {
                let wall = if boundary {
                    ' '
                } else {
                    match distance_to_wall {
                        d if d <= DEPTH / 4.0 => '\u{2588}',
                        d if d < DEPTH / 3.0 => '\u{2593}',
                        d if d < DEPTH / 2.0 => '\u{2592}',
                        d if d < DEPTH => '\u{2591}',
                        _ => ' ',
                    }
                };

                wall as u16
            } else {
                let floor_distance =
                    1.0 - (y as f32 - SCREEN_HEIGHT as f32 / 2.0) / (SCREEN_HEIGHT as f32 / 2.0);
                let floor = match floor_distance {
                    fd if fd < 0.25 => '#',
                    fd if fd < 0.5 => 'x',
                    fd if fd < 0.75 => '-',
                    fd if fd < 0.9 => '.',
                    _ => ' ',
                };
                floor as u16
            };
        }
    }
}

fn draw_screen_to_console(h_console: HANDLE, screen: &mut Vec<u16>, bytes_written: &mut u32) {
    screen[SCREEN_SIZE - 1] = '\0' as u16;
    unsafe {
        WriteConsoleOutputCharacterW(
            h_console,
            &screen[0],
            SCREEN_SIZE as u32,
            COORD { X: 0, Y: 0 },
            bytes_written,
        );
    }
}
