#![cfg(target_os = "windows")]

use std::{
    env,
    mem::{size_of, zeroed},
    process,
    thread,
    time::Duration,
};

use winapi::{
    shared::windef::POINT,
    um::winuser::{
        GetAsyncKeyState, GetCursorPos, GetSystemMetrics, SendInput, SetCursorPos, INPUT,
        INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
        MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
        MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, SM_CXSCREEN, SM_CYSCREEN,
        VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
    },
};

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();

    let command = args
        .first()
        .ok_or_else(|| "Missing command.".to_string())?;

    match command.as_str() {
        "move" => {
            require_len(&args, 3)?;
            let x = parse_i32(&args[1], "x")?;
            let y = parse_i32(&args[2], "y")?;

            unsafe {
                if SetCursorPos(x, y) == 0 {
                    return Err("SetCursorPos failed.".to_string());
                }
            }

            Ok(0)
        }

        "move-relative" => {
            require_len(&args, 3)?;
            let x = parse_i32(&args[1], "x")?;
            let y = parse_i32(&args[2], "y")?;

            send_mouse(x, y, 0, MOUSEEVENTF_MOVE)?;
            Ok(0)
        }

        "click" => {
            require_len(&args, 2)?;
            mouse_button(&args[1], true)?;
            mouse_button(&args[1], false)?;
            Ok(0)
        }

        "double-click" => {
            require_len(&args, 2)?;

            for _ in 0..2 {
                mouse_button(&args[1], true)?;
                mouse_button(&args[1], false)?;
                thread::sleep(Duration::from_millis(60));
            }

            Ok(0)
        }

        "mouse-down" => {
            require_len(&args, 2)?;
            mouse_button(&args[1], true)?;
            Ok(0)
        }

        "mouse-up" => {
            require_len(&args, 2)?;
            mouse_button(&args[1], false)?;
            Ok(0)
        }

        "mouse-pressed" => {
            require_len(&args, 2)?;
            let key = mouse_virtual_key(&args[1])?;
            Ok(if key_pressed(key) { 0 } else { 1 })
        }

        "scroll" => {
            require_len(&args, 2)?;
            let amount = parse_i32(&args[1], "amount")?;
            let wheel_delta = amount.saturating_mul(120);

            send_mouse(0, 0, wheel_delta as u32, MOUSEEVENTF_WHEEL)?;
            Ok(0)
        }

        "key-tap" => {
            require_len(&args, 2)?;
            let key = virtual_key(&args[1])?;

            send_key(key, false)?;
            send_key(key, true)?;
            Ok(0)
        }

        "key-down" => {
            require_len(&args, 2)?;
            let key = virtual_key(&args[1])?;

            send_key(key, false)?;
            Ok(0)
        }

        "key-up" => {
            require_len(&args, 2)?;
            let key = virtual_key(&args[1])?;

            send_key(key, true)?;
            Ok(0)
        }

        "key-pressed" => {
            require_len(&args, 2)?;
            let key = virtual_key(&args[1])?;

            Ok(if key_pressed(key) { 0 } else { 1 })
        }

        "type" => {
            require_len(&args
