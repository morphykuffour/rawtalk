extern crate hidapi;

use hidapi::HidApi;
use std::fs::File;
use std::io::{self, BufRead};

const VIM_MODE_FILE: &str = "/tmp/vim_mode";
const BUFFER_SIZE: usize = 32;

#[derive(Debug)]
enum Command {
    GetCurrentLayer = 0x40,
    Layer0 = 0x30,
    Layer1 = 0x31,
    Layer2 = 0x32,
    Layer3 = 0x33,
}

fn send_command(device: &hidapi::HidDevice, cmd: Command) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let command = [cmd as u8, 0x00];  // Command format from auto_layers
    println!("Sending command: {:02X?}", command);
    
    device.write(&command)?;
    
    let mut buf = [0u8; BUFFER_SIZE];
    match device.read_timeout(&mut buf, 1000) {
        Ok(len) => {
            println!("Received response ({} bytes): {:02X?}", len, &buf[..len]);
            if len > 0 {
                Ok(Some(buf[..len].to_vec()))
            } else {
                Ok(None)  // No data received
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
        let cmd = match line?.as_str() {
            "n" => Command::Layer0,  // Normal mode
            "i" => Command::Layer1,  // Insert mode
            "v" => Command::Layer2,  // Visual mode
            "c" => Command::Layer3,  // Command mode
            _ => continue,
        };

        match send_command(&device, cmd) {
            Ok(Some(_)) => println!("Mode change successful"),
            Ok(None) => println!("No response received from keyboard"),
            Err(e) => eprintln!("Failed to change mode: {}", e),
        }
    }

    Ok(())
}
