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

fn send_layer_command(device: &hidapi::HidDevice, layer: u8) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut command_buffer = vec![0u8; 32];
    command_buffer[0] = 0x01;  // Command ID for layer switch
    command_buffer[1] = layer; // Target layer
    command_buffer[2] = 0xAA;  // Verification byte
    
    println!("Sending layer command: {:02X?}", command_buffer);
    match device.write(&command_buffer) {
        Ok(len) => println!("Wrote {} bytes", len),
        Err(e) => println!("Write error: {}", e),
    }
    
    let mut buf = vec![0u8; 32];
    match device.read_timeout(&mut buf, 1000) {
        Ok(len) => {
            if len > 0 {
                println!("Received response ({} bytes): {:02X?}", len, &buf[..len]);
                Ok(Some(buf[..len].to_vec()))
            } else {
                println!("No data received (timeout)");
                Ok(None)
            }
        },
        Err(e) => {
            println!("Read error: {}", e);
            Err(Box::new(e))
        },
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    let (vid, pid) = (0xC2AB, 0x3939);
    
    let device_info = api.device_list()
        .find(|d| d.vendor_id() == vid 
            && d.product_id() == pid
            && d.usage_page() == 0xFF60)
        .ok_or("Could not find Raw HID interface")?;

    let device = device_info.open_device(&api)?;
    println!("Connected to Ferris sweep Raw HID interface!");

    // Continuously monitor vim_mode file
    let mut file = File::open(get_vim_mode_path())?;
    let mut last_mode = String::new();
    
    // Read initial vim mode
    let mut current_mode = String::new();
    let mut reader = io::BufReader::new(&file);
    reader.read_line(&mut current_mode)?;
    let current_mode = current_mode.trim();
    println!("Initial vim mode: '{}'", current_mode);
    
    // Set initial layer based on current mode
    let layer = match current_mode {
        "i" => {
            println!("Detected insert mode, setting layer 0");
            0u8
        },
        "n" | "v" | "c" => {
            println!("Detected normal/visual/command mode, setting layer 3");
            3u8
        },
        _ => {
            println!("Unknown mode '{}', defaulting to layer 0", current_mode);
            0u8
        },
    };
    
    // Send initial layer command
    println!("Sending initial layer command for layer {}", layer);
    match send_layer_command(&device, layer) {
        Ok(Some(response)) => {
            match (response[0], response[1], response[2]) {
                (0x00, layer, 0xAA) => println!("Initial layer set to: {}", layer),
                _ => println!("Unexpected response: {:02X?}", &response[..3]),
            }
        },
        Ok(None) => println!("No response received"),
        Err(e) => eprintln!("Error: {}", e),
    }
    
    last_mode = current_mode.to_string();
    
    loop {
        // Reset file position to start
        file.seek(SeekFrom::Start(0))?;
        
        let mut current_mode = String::new();
        let mut reader = io::BufReader::new(&file);
        reader.read_line(&mut current_mode)?;
        let current_mode = current_mode.trim();
        
        // Only process and print when mode changes
        if current_mode != last_mode {
            println!("Mode changed: '{}' -> '{}'", last_mode, current_mode);
            
            let layer = match current_mode {
                "i" => {
                    println!("Switching to insert mode (layer 0)");
                    0u8
                },
                "n" | "v" | "c" => {
                    println!("Switching to normal/visual/command mode (layer 3)");
                    3u8
                },
                _ => {
                    println!("Unknown mode '{}', skipping", current_mode);
                    continue;
                },
            };

            match send_layer_command(&device, layer) {
                Ok(Some(response)) => {
                    match (response[0], response[1], response[2]) {
                        (0x00, layer, 0xAA) => println!("Layer switch successful (layer: {})", layer),
                        _ => println!("Unexpected response: {:02X?}", &response[..3]),
                    }
                },
                Ok(None) => println!("No response received"),
                Err(e) => eprintln!("Error: {}", e),
            }
            
            last_mode = current_mode.to_string();
        }
        
        // Small delay to prevent busy-waiting
        sleep(Duration::from_millis(100));
    }
}
