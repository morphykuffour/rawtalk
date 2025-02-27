extern crate hidapi;

use hidapi::HidApi;
use std::fs::File;
use std::io::{self, BufRead, Seek, SeekFrom};
use std::thread::sleep;
use std::time::Duration;
use std::env;
use std::path::PathBuf;

const BUFFER_SIZE: usize = 32;
const RAW_EPSIZE: usize = 32;  // Must match QMK's RAW_EPSIZE

#[derive(Debug)]
enum Command {
    GetCurrentLayer = 0x40,
    Layer0 = 0x30,
    Layer1 = 0x31,
    Layer2 = 0x32,
    Layer3 = 0x33,
}

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

fn send_command(device: &hidapi::HidDevice, cmd: Command) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    // Create a properly sized buffer
    let mut command_buffer = vec![0u8; RAW_EPSIZE];
    command_buffer[0] = cmd as u8;  // First byte is command
    
    println!("Sending command: {:02X?}", command_buffer);
    
    // Write full buffer
    device.write(&command_buffer)?;
    
    // Read with correct buffer size
    let mut buf = vec![0u8; RAW_EPSIZE];
    match device.read_timeout(&mut buf, 1000) {
        Ok(len) => {
            println!("Received response ({} bytes): {:02X?}", len, &buf[..len]);
            if len > 0 {
                Ok(Some(buf[..len].to_vec()))
            } else {
                Ok(None)
            }
        },
        Err(e) => Err(Box::new(e)),
    }
}

fn find_keyboard(api: &HidApi) -> Result<hidapi::HidDevice, Box<dyn std::error::Error>> {
    const KEYBOARD_VID: u16 = 0xC2AB;
    const KEYBOARD_PID: u16 = 0x3939;
    const USAGE_PAGE: u16 = 0xFF60;  // QMK's usage page
    const USAGE: u16 = 0x61;         // QMK's usage
    
    for device in api.device_list() {
        if device.vendor_id() == KEYBOARD_VID 
           && device.product_id() == KEYBOARD_PID 
           && device.usage_page() == USAGE_PAGE 
           && device.usage() == USAGE {
            match device.open_device(api) {
                Ok(dev) => {
                    println!("Found keyboard at path: {}", device.path().to_string_lossy());
                    return Ok(dev)
                },
                Err(e) => {
                    println!("Found keyboard but failed to open: {}", e);
                    continue;
                }
            }
        }
    }
    
    Err("Keyboard not found or access denied. Make sure you have the right permissions.".into())
}

fn send_layer_command(device: &hidapi::HidDevice, layer: u8) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut command_buffer = vec![0u8; 32];
    command_buffer[0] = 0x00;  // Layer switch command
    command_buffer[1] = layer; // Target layer (0-3)
    
    println!("Sending layer command: {:02X?}", command_buffer);
    
    match device.write(&command_buffer) {
        Ok(written) => {
            println!("Wrote {} bytes", written);
            if written != 32 {
                println!("Warning: Expected to write 32 bytes, wrote {}", written);
            }
        },
        Err(e) => {
            println!("Failed to write to device: {}", e);
            return Err(e.into());
        }
    }
    
    sleep(Duration::from_millis(50));
    
    let mut response = vec![0u8; 32];
    match device.read_timeout(&mut response, 1000) {
        Ok(len) => {
            if len > 0 {
                println!("Received response ({} bytes): {:02X?}", len, &response[..len]);
                match response[0] {
                    0x00 => {
                        println!("Layer switched successfully to colemak-dh, new layer: {}", response[1]);
                        Ok(Some(response[..len].to_vec()))
                    },
                    0xFF => {
                        println!("Layer switched successfully to qwerty");
                        Ok(Some(response[..len].to_vec()))
                    },
                    _ => {
                        println!("Unexpected response code: 0x{:02X}", response[0]);
                        Ok(Some(response[..len].to_vec()))
                    }
                }
            } else {
                println!("No data received (timeout)");
                Ok(None)
            }
        },
        Err(e) => {
            println!("Read error: {}", e);
            Err(e.into())
        },
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    
    // Find our keyboard
    let device = find_keyboard(&api)?;
    println!("Connected to Ferris sweep Raw HID interface!");

    let mode_path = get_vim_mode_path();
    let mut file = File::open(&mode_path)?;
    
    // Read initial mode
    let mut current_mode = String::new();
    let mut reader = io::BufReader::new(&file);
    reader.read_line(&mut current_mode)?;
    let current_mode = current_mode.trim();
    println!("Initial vim mode: '{}'", current_mode);
    
    // Set initial layer based on current mode
    let layer = match current_mode {
        "i" => {
            println!("Detected insert mode, setting layer 3 (colemak-dh)");
            3u8
        },
        _ => {
            println!("Detected other mode, setting layer 1 (qwerty)");
            1u8
        },
    };
    
    println!("Sending initial layer command for layer {}", layer);
    send_layer_command(&device, layer)?;
    
    let mut last_mode = current_mode.to_string();
    
    loop {
        file.seek(SeekFrom::Start(0))?;
        
        let mut current_mode = String::new();
        let mut reader = io::BufReader::new(&file);
        reader.read_line(&mut current_mode)?;
        let current_mode = current_mode.trim();
        
        if current_mode != last_mode {
            println!("Mode changed: '{}' -> '{}'", last_mode, current_mode);
            
            let layer = match current_mode {
                "i" => {
                    println!("Switching to insert mode (layer 3 - colemak-dh)");
                    3u8
                },
                _ => {
                    println!("Switching to other mode (layer 1 - qwerty)");
                    1u8
                },
            };

            send_layer_command(&device, layer)?;
            last_mode = current_mode.to_string();
        }
        
        sleep(Duration::from_millis(100));
    }
}
