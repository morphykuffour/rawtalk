extern crate hidapi;

use hidapi::HidApi;
use std::fs::File;
use std::io::{self, BufRead};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    let (VID, PID) = (0xC2AB, 0x3939);
    
    // Find the correct interface
    let device_info = api.device_list()
        .find(|d| d.vendor_id() == VID 
            && d.product_id() == PID
            && d.usage_page() == 0xFF60)
        .ok_or("Could not find Raw HID interface")?;

    let device = device_info.open_device(&api)?;
    println!("Connected to Ferris sweep Raw HID interface!");

    // Test connection by getting current layer
    println!("Testing connection by getting current layer...");
    match send_command(&device, Command::GetCurrentLayer) {
        Ok(Some(response)) => println!("Current layer: {}", response[0]),
        Ok(None) => println!("No response received from keyboard"),
        Err(e) => eprintln!("Failed to get current layer: {}", e),
    }

    // Watch for mode changes
    let file = File::open(VIM_MODE_FILE)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let mode = match line?.as_str() {
            "i" => [0x00, 0x00],  // Insert mode -> Layer 0
            "n" | "v" | "c" => [0x00, 0x03],  // All other modes -> Layer 3
            _ => continue,
        };

        // Send the command
        match device.write(&mode) {
            Ok(_) => {
                println!("Sent mode change command: {:02X?}", mode);
                
                // Read the response
                let mut buf = [0u8; BUFFER_SIZE];
                match device.read_timeout(&mut buf, 100) {
                    Ok(len) => {
                        println!("Received response ({} bytes): {:02X?}", len, &buf[..len]);
                        match (buf[0], buf[1], buf[2]) {
                            (0x00, m, 0xAA) => println!("Mode change successful (mode: {:02X})", m),
                            _ => println!("Unexpected response"),
                        }
                    },
                    Err(e) => eprintln!("Failed to read response: {}", e),
                }
            },
            Err(e) => eprintln!("Failed to send command: {}", e),
        }
    }

    Ok(())
}
