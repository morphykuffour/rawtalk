extern crate hidapi;

use hidapi::HidApi;
use std::fs::File;
use std::io::{self, BufRead, Seek, SeekFrom};
use std::thread::sleep;
use std::time::Duration;

const VIM_MODE_FILE: &str = "/tmp/vim_mode";
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
    // Create full-size buffer (32 bytes)
    let mut command_buffer = vec![0u8; RAW_EPSIZE];
    command_buffer[0] = 0x00;  // Layer switch command
    command_buffer[1] = layer; // Target layer
    
    println!("Sending layer command: {:02X?}", command_buffer);
    
    // Write full buffer
    device.write(&command_buffer)?;
    
    // Read response
    let mut buf = vec![0u8; RAW_EPSIZE];
    match device.read_timeout(&mut buf, 100) {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    let (VID, PID) = (0xC2AB, 0x3939);
    
    let device_info = api.device_list()
        .find(|d| d.vendor_id() == VID 
            && d.product_id() == PID
            && d.usage_page() == 0xFF60)
        .ok_or("Could not find Raw HID interface")?;

    let device = device_info.open_device(&api)?;
    println!("Connected to Ferris sweep Raw HID interface!");

    // Test connection
    println!("Testing connection by getting current layer...");
    match send_command(&device, Command::GetCurrentLayer) {
        Ok(Some(response)) => println!("Current layer: {}", response[0]),
        Ok(None) => println!("No response received from keyboard"),
        Err(e) => eprintln!("Failed to get current layer: {}", e),
    }

    // Continuously monitor vim_mode file
    let mut file = File::open(VIM_MODE_FILE)?;
    let mut last_mode = String::new();
    
    loop {
        // Reset file position to start
        file.seek(SeekFrom::Start(0))?;
        
        let mut current_mode = String::new();
        let mut reader = io::BufReader::new(&file);
        reader.read_line(&mut current_mode)?;
        
        // Trim whitespace and newlines
        let current_mode = current_mode.trim();
        
        // Only send command if mode changed
        if current_mode != last_mode {
            println!("Mode changed from '{}' to '{}'", last_mode, current_mode);
            
            let layer = match current_mode {
                "i" => 0u8,  // Insert mode -> Layer 0
                "n" | "v" | "c" => 3u8,  // All other modes -> Layer 3
                _ => continue,
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
