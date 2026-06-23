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
            Ok(if key_pressed(i32::from(key)) { 0 } else { 1 })
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

            Ok(if key_pressed(i32::from(key)) { 0 } else { 1 })
        }

        "type" => {
            require_len(&args, 2)?;
            type_text(&args[1])?;
            Ok(0)
        }

        "hotkey" => {
            if args.len() < 2 {
                return Err("hotkey requires at least one key.".to_string());
            }

            let keys: Vec<u16> = args[1..]
                .iter()
                .map(|name| virtual_key(name))
                .collect::<Result<_, _>>()?;

            for key in &keys {
                send_key(*key, false)?;
            }

            for key in keys.iter().rev() {
                send_key(*key, true)?;
            }

            Ok(0)
        }

        "read-key" => {
            wait_for_key();
            Ok(0)
        }

        "mouse-position" => {
            let mut point: POINT = unsafe { zeroed() };

            unsafe {
                if GetCursorPos(&mut point) == 0 {
                    return Err("GetCursorPos failed.".to_string());
                }
            }

            println!("{},{}", point.x, point.y);
            Ok(0)
        }

        "screen-size" => {
            let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

            println!("{width},{height}");
            Ok(0)
        }

        "help" | "--help" | "-h" => {
            print_help();
            Ok(0)
        }

        other => Err(format!("Unknown command: {other}")),
    }
}

fn require_len(args: &[String], expected: usize) -> Result<(), String> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(format!(
            "{} expected {} arguments, got {}.",
            args[0],
            expected - 1,
            args.len().saturating_sub(1)
        ))
    }
}

fn parse_i32(value: &str, name: &str) -> Result<i32, String> {
    value
        .parse::<i32>()
        .map_err(|_| format!("Invalid {name}: {value}"))
}

fn send_mouse(
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: u32,
) -> Result<(), String> {
    unsafe {
        let mut input: INPUT = zeroed();
        input.type_ = INPUT_MOUSE;

        *input.u.mi_mut() = MOUSEINPUT {
            dx,
            dy,
            mouseData: mouse_data,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: 0,
        };

        let sent = SendInput(
            1,
            &mut input,
            size_of::<INPUT>() as i32,
        );

        if sent != 1 {
            return Err("SendInput failed for mouse input.".to_string());
        }
    }

    Ok(())
}

fn mouse_button(button: &str, down: bool) -> Result<(), String> {
    let normalized = button.trim().to_ascii_lowercase();

    let flag = match (normalized.as_str(), down) {
        ("left", true) => MOUSEEVENTF_LEFTDOWN,
        ("left", false) => MOUSEEVENTF_LEFTUP,

        ("right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("right", false) => MOUSEEVENTF_RIGHTUP,

        ("middle", true) => MOUSEEVENTF_MIDDLEDOWN,
        ("middle", false) => MOUSEEVENTF_MIDDLEUP,

        _ => return Err(format!("Unknown mouse button: {button}")),
    };

    send_mouse(0, 0, 0, flag)
}

fn mouse_virtual_key(button: &str) -> Result<i32, String> {
    match button.trim().to_ascii_lowercase().as_str() {
        "left" => Ok(VK_LBUTTON),
        "right" => Ok(VK_RBUTTON),
        "middle" => Ok(VK_MBUTTON),
        _ => Err(format!("Unknown mouse button: {button}")),
    }
}

fn send_key(key: u16, key_up: bool) -> Result<(), String> {
    unsafe {
        let mut input: INPUT = zeroed();
        input.type_ = INPUT_KEYBOARD;

        *input.u.ki_mut() = KEYBDINPUT {
            wVk: key,
            wScan: 0,
            dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
            time: 0,
            dwExtraInfo: 0,
        };

        let sent = SendInput(
            1,
            &mut input,
            size_of::<INPUT>() as i32,
        );

        if sent != 1 {
            return Err("SendInput failed for keyboard input.".to_string());
        }
    }

    Ok(())
}

fn send_unicode(code_unit: u16, key_up: bool) -> Result<(), String> {
    unsafe {
        let mut input: INPUT = zeroed();
        input.type_ = INPUT_KEYBOARD;

        *input.u.ki_mut() = KEYBDINPUT {
            wVk: 0,
            wScan: code_unit,
            dwFlags: KEYEVENTF_UNICODE
                | if key_up { KEYEVENTF_KEYUP } else { 0 },
            time: 0,
            dwExtraInfo: 0,
        };

        let sent = SendInput(
            1,
            &mut input,
            size_of::<INPUT>() as i32,
        );

        if sent != 1 {
            return Err("SendInput failed while typing text.".to_string());
        }
    }

    Ok(())
}

fn type_text(text: &str) -> Result<(), String> {
    for code_unit in text.encode_utf16() {
        send_unicode(code_unit, false)?;
        send_unicode(code_unit, true)?;
    }

    Ok(())
}

fn key_pressed(key: i32) -> bool {
    unsafe { (GetAsyncKeyState(key) as u16 & 0x8000) != 0 }
}

fn virtual_key(name: &str) -> Result<u16, String> {
    let key = name.trim().to_ascii_uppercase();

    if key.len() == 1 {
        let value = key.as_bytes()[0];

        if value.is_ascii_alphanumeric() {
            return Ok(value as u16);
        }
    }

    if let Some(number) = key.strip_prefix('F') {
        if let Ok(number) = number.parse::<u16>() {
            if (1..=24).contains(&number) {
                return Ok(0x70 + number - 1);
            }
        }
    }

    let value = match key.as_str() {
        "BACKSPACE" => 0x08,
        "TAB" => 0x09,
        "ENTER" | "RETURN" => 0x0D,

        "SHIFT" => 0x10,
        "CTRL" | "CONTROL" => 0x11,
        "ALT" => 0x12,

        "PAUSE" => 0x13,
        "CAPSLOCK" => 0x14,
        "ESC" | "ESCAPE" => 0x1B,
        "SPACE" => 0x20,

        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        "END" => 0x23,
        "HOME" => 0x24,

        "LEFT" => 0x25,
        "UP" => 0x26,
        "RIGHT" => 0x27,
        "DOWN" => 0x28,

        "PRINTSCREEN" => 0x2C,
        "INSERT" => 0x2D,
        "DELETE" => 0x2E,

        "LWIN" | "WIN" | "WINDOWS" => 0x5B,
        "RWIN" => 0x5C,

        "NUM0" => 0x60,
        "NUM1" => 0x61,
        "NUM2" => 0x62,
        "NUM3" => 0x63,
        "NUM4" => 0x64,
        "NUM5" => 0x65,
        "NUM6" => 0x66,
        "NUM7" => 0x67,
        "NUM8" => 0x68,
        "NUM9" => 0x69,

        "MULTIPLY" => 0x6A,
        "ADD" => 0x6B,
        "SUBTRACT" => 0x6D,
        "DECIMAL" => 0x6E,
        "DIVIDE" => 0x6F,

        "NUMLOCK" => 0x90,
        "SCROLLLOCK" => 0x91,

        _ => return Err(format!("Unknown key: {name}")),
    };

    Ok(value)
}

fn wait_for_key() {
    loop {
        for key in 0x08..=0xFE {
            if key_pressed(key) {
                println!("{}", key_name(key as u16));

                while key_pressed(key) {
                    thread::sleep(Duration::from_millis(10));
                }

                return;
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn key_name(key: u16) -> String {
    match key {
        0x08 => "BACKSPACE".to_string(),
        0x09 => "TAB".to_string(),
        0x0D => "ENTER".to_string(),
        0x10 => "SHIFT".to_string(),
        0x11 => "CTRL".to_string(),
        0x12 => "ALT".to_string(),
        0x1B => "ESC".to_string(),
        0x20 => "SPACE".to_string(),

        0x25 => "LEFT".to_string(),
        0x26 => "UP".to_string(),
        0x27 => "RIGHT".to_string(),
        0x28 => "DOWN".to_string(),

        0x2D => "INSERT".to_string(),
        0x2E => "DELETE".to_string(),

        0x30..=0x39 | 0x41..=0x5A => {
            char::from_u32(key as u32)
                .unwrap_or('?')
                .to_string()
        }

        0x70..=0x87 => format!("F{}", key - 0x70 + 1),

        _ => format!("VK_{key:02X}"),
    }
}

fn print_help() {
    println!("TSInput Driver");
    println!();
    println!("Commands:");
    println!("  move X Y");
    println!("  move-relative X Y");
    println!("  click BUTTON");
    println!("  double-click BUTTON");
    println!("  mouse-down BUTTON");
    println!("  mouse-up BUTTON");
    println!("  mouse-pressed BUTTON");
    println!("  scroll AMOUNT");
    println!("  key-tap KEY");
    println!("  key-down KEY");
    println!("  key-up KEY");
    println!("  key-pressed KEY");
    println!("  type TEXT");
    println!("  hotkey KEY...");
    println!("  read-key");
    println!("  mouse-position");
    println!("  screen-size");
}
