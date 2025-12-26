extern crate hidapi;

use hidapi::HidApi;
use std::fs::File;
use std::io::{self, BufRead, Seek, SeekFrom};
use std::thread::sleep;
use std::time::Duration;
use std::env;
use std::path::PathBuf;

const RAW_EPSIZE: usize = 32;  // Must match QMK's RAW_EPSIZE

fn get_vim_mode_path() -> PathBuf {
    // First check if VIM_MODE_FILE env var is set
    if let Ok(path) = env::var("VIM_MODE_FILE") {
        return PathBuf::from(path);
    }

    // Fall back to default locations based on OS
    if cfg!(windows) {
        PathBuf::from(r"C:\tmp\vim_mode.txt")
    } else {
        PathBuf::from("/tmp/vim_mode")
    }
}

/// Send a raw HID command to the keyboard.
/// 
/// On Linux and macOS, hidapi requires prepending a 0x00 report ID byte
/// for devices that don't use numbered reports. Windows handles this
/// automatically, so we need platform-specific handling.
fn send_raw_hid(device: &hidapi::HidDevice, data: &[u8]) -> Result<usize, hidapi::HidError> {
    #[cfg(target_os = "windows")]
    {
        // Windows: send data directly, hidapi handles report ID internally
        device.write(data)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Linux/macOS: prepend 0x00 report ID for devices without numbered reports
        let mut buf = vec![0x00u8];  // Report ID 0
        buf.extend_from_slice(data);
        device.write(&buf)
    }
}

fn find_keyboard(api: &HidApi) -> Result<hidapi::HidDevice, Box<dyn std::error::Error>> {
    const KEYBOARD_VID: u16 = 0xC2AB;
    const KEYBOARD_PID: u16 = 0x3939;
    const USAGE_PAGE: u16 = 0xFF60;  // QMK's usage page
    const USAGE: u16 = 0x61;         // QMK's usage

    println!("Searching for keyboard VID={:04X} PID={:04X}...", KEYBOARD_VID, KEYBOARD_PID);

    for device in api.device_list() {
        // Debug: print all HID devices with matching VID/PID
        if device.vendor_id() == KEYBOARD_VID && device.product_id() == KEYBOARD_PID {
            println!(
                "Found matching VID/PID - Usage Page: {:04X}, Usage: {:02X}, Interface: {}, Path: {}",
                device.usage_page(),
                device.usage(),
                device.interface_number(),
                device.path().to_string_lossy()
            );
        }

        if device.vendor_id() == KEYBOARD_VID
           && device.product_id() == KEYBOARD_PID
           && device.usage_page() == USAGE_PAGE
           && device.usage() == USAGE {
            match device.open_device(api) {
                Ok(dev) => {
                    println!("Successfully opened keyboard at path: {}", device.path().to_string_lossy());
                    return Ok(dev)
                },
                Err(e) => {
                    eprintln!("Found keyboard but failed to open: {}", e);
                    eprintln!("Hint: On Linux, you may need udev rules. On macOS, check System Preferences > Security & Privacy.");
                    continue;
                }
            }
        }
    }

    // If we didn't find with usage page filter, try without (for debugging)
    println!("\nDebug: Listing all devices with matching VID/PID:");
    for device in api.device_list() {
        if device.vendor_id() == KEYBOARD_VID && device.product_id() == KEYBOARD_PID {
            println!(
                "  - Usage Page: {:04X}, Usage: {:02X}, Interface: {}, Path: {}",
                device.usage_page(),
                device.usage(),
                device.interface_number(),
                device.path().to_string_lossy()
            );
        }
    }

    Err("Keyboard not found or access denied. Make sure you have the right permissions.\n\
         On Linux: Install udev rules and run: sudo udevadm control --reload-rules && sudo udevadm trigger\n\
         On macOS: Grant Input Monitoring permission in System Preferences > Security & Privacy > Privacy".into())
}

fn send_layer_command(device: &hidapi::HidDevice, layer: u8) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut command_buffer = vec![0u8; RAW_EPSIZE];
    command_buffer[0] = 0x00;  // Layer switch command
    command_buffer[1] = layer; // Target layer (0-3)

    println!("Sending layer {} command...", layer);

    match send_raw_hid(device, &command_buffer) {
        Ok(written) => {
            println!("Wrote {} bytes", written);
        },
        Err(e) => {
            eprintln!("Failed to write to device: {}", e);
            return Err(e.into());
        }
    }

    // Small delay to allow keyboard to process
    sleep(Duration::from_millis(50));

    let mut response = vec![0u8; RAW_EPSIZE];
    match device.read_timeout(&mut response, 1000) {
        Ok(len) => {
            if len > 0 {
                println!("Received response ({} bytes): {:02X?}", len, &response[..len]);
                match response[0] {
                    0x00 => {
                        println!("Success: Layer {} active", response[1]);
                        Ok(Some(response[..len].to_vec()))
                    },
                    0xFF => {
                        println!("Error response from keyboard");
                        Ok(Some(response[..len].to_vec()))
                    },
                    _ => {
                        println!("Unexpected response code: 0x{:02X}", response[0]);
                        Ok(Some(response[..len].to_vec()))
                    }
                }
            } else {
                println!("No response received (timeout) - command may still have succeeded");
                Ok(None)
            }
        },
        Err(e) => {
            eprintln!("Read error: {}", e);
            Err(e.into())
        },
    }
}

fn get_current_layer(device: &hidapi::HidDevice) -> Result<u8, Box<dyn std::error::Error>> {
    let mut command_buffer = vec![0u8; RAW_EPSIZE];
    command_buffer[0] = 0x40;  // Get current layer command

    send_raw_hid(device, &command_buffer)?;

    let mut response = vec![0u8; RAW_EPSIZE];
    match device.read_timeout(&mut response, 1000) {
        Ok(len) if len > 0 => Ok(response[0]),
        Ok(_) => Err("No response when querying layer".into()),
        Err(e) => Err(e.into()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("QMK Layer Switcher - Neovim Mode Sync");
    println!("=====================================");

    let api = HidApi::new()?;

    // Find our keyboard
    let device = find_keyboard(&api)?;
    println!("Connected to Ferris Sweep Raw HID interface!");

    // Try to get current layer
    match get_current_layer(&device) {
        Ok(layer) => println!("Current keyboard layer: {}", layer),
        Err(e) => println!("Could not query current layer: {}", e),
    }

    let mode_path = get_vim_mode_path();
    println!("Watching vim mode file: {}", mode_path.display());

    let mut file = match File::open(&mode_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open vim mode file '{}': {}", mode_path.display(), e);
            eprintln!("Make sure Neovim is writing the mode to this file.");
            eprintln!("You can set VIM_MODE_FILE environment variable to customize the path.");
            return Err(e.into());
        }
    };

    // Read initial mode
    let mut current_mode = String::new();
    let mut reader = io::BufReader::new(&file);
    reader.read_line(&mut current_mode)?;
    let current_mode = current_mode.trim();
    println!("Initial vim mode: '{}'", current_mode);

    // Layer mapping:
    // - Layer 0: Colemak-DH (for insert mode)
    // - Layer 3: QWERTY (for normal/other modes)
    // Note: Adjust these layer numbers based on your keymap.c configuration
    let layer = if current_mode == "i" {
        println!("Detected insert mode -> Colemak-DH (layer 0)");
        0u8
    } else {
        println!("Detected normal/other mode -> QWERTY (layer 3)");
        3u8
    };

    send_layer_command(&device, layer)?;

    let mut last_mode = current_mode.to_string();

    println!("\nMonitoring for mode changes...");
    
    loop {
        file.seek(SeekFrom::Start(0))?;

        let mut current_mode = String::new();
        let mut reader = io::BufReader::new(&file);
        reader.read_line(&mut current_mode)?;
        let current_mode = current_mode.trim();

        if current_mode != last_mode {
            println!("Mode changed: '{}' -> '{}'", last_mode, current_mode);

            let layer = if current_mode == "i" {
                println!("Switching to Colemak-DH (layer 0)");
                0u8
            } else {
                println!("Switching to QWERTY (layer 3)");
                3u8
            };

            if let Err(e) = send_layer_command(&device, layer) {
                eprintln!("Failed to switch layer: {}", e);
                // Don't exit, try again next iteration
            }
            last_mode = current_mode.to_string();
        }

        sleep(Duration::from_millis(100));
    }
}
