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
    const VID: u16 = 0xC2AB;
    const PID: u16 = 0x3939;
    const USAGE_PAGE: u16 = 0xFF60;
    
    println!("Searching for keyboard (VID: {:04X}, PID: {:04X}, Usage Page: {:04X})", VID, PID, USAGE_PAGE);
    
    // List all HID devices first
    println!("\nAvailable HID devices:");
    for device in api.device_list() {
        println!("Device: VID={:04X}, PID={:04X}, Usage Page={:04X}, Path={}",
            device.vendor_id(),
            device.product_id(),
            device.usage_page(),
            device.path().to_string_lossy()
        );
    }
    
    // Now try to find our specific device
    for device in api.device_list() {
        if device.vendor_id() == VID 
           && device.product_id() == PID 
           && device.usage_page() == USAGE_PAGE {
            
            println!("\nFound matching keyboard:");
            println!("Path: {}", device.path().to_string_lossy());
            println!("Manufacturer: {}", device.manufacturer_string().unwrap_or("Unknown"));
            println!("Product: {}", device.product_string().unwrap_or("Unknown"));
            println!("Serial: {}", device.serial_number().unwrap_or("Unknown"));
            println!("Interface: {}", device.interface_number());
            println!("Usage Page: {:04X}", device.usage_page());
            
            match device.open_device(api) {
                Ok(dev) => {
                    #[cfg(target_os = "macos")]
                    {
                        println!("Setting macOS-specific device options...");
                        // Try both blocking and non-blocking modes
                        if let Err(e) = dev.set_blocking_mode(true) {
                            println!("Warning: Could not set blocking mode: {}", e);
                        }
                    }
                    println!("Successfully opened device!");
                    return Ok(dev);
                },
                Err(e) => {
                    println!("Found keyboard but failed to open: {}", e);
                    continue;
                }
            }
        }
    }
    
    Err("Keyboard not found or access denied. Available devices listed above.".into())
}

fn send_layer_command(device: &hidapi::HidDevice, layer: u8) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut command_buffer = vec![0u8; RAW_EPSIZE];
    command_buffer[0] = 0x00;  // Layer switch command
    command_buffer[1] = layer; // Target layer
    
    println!("Sending layer command: {:02X?}", command_buffer);
    match device.write(&command_buffer) {
        Ok(len) => println!("Wrote {} bytes", len),
        Err(e) => {
            println!("Write error: {}", e);
            return Err(Box::new(e));
        }
    }
    
    // Try multiple reads with different timeouts
    for timeout in [100, 500, 1000] {
        println!("Attempting read with {}ms timeout...", timeout);
        let mut buf = vec![0u8; RAW_EPSIZE];
        match device.read_timeout(&mut buf, timeout) {
            Ok(len) if len > 0 => {
                println!("Received response ({} bytes): {:02X?}", len, &buf[..len]);
                return Ok(Some(buf[..len].to_vec()));
            }
            Ok(_) => continue,
            Err(e) => {
                println!("Read error ({}ms): {}", timeout, e);
                continue;
            }
        }
    }
    
    println!("No response after multiple attempts");
    Ok(None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    let (VID, PID) = (0xC2AB, 0x3939);
    
    // Platform-specific device initialization
    let device = match find_keyboard(&api) {
        Ok(dev) => dev,
        Err(e) => {
            eprintln!("Failed to open keyboard: {}", e);
            eprintln!("On Linux/macOS, ensure you have the correct udev rules/permissions:");
            eprintln!("Linux: Add udev rule to /etc/udev/rules.d/50-qmk.rules:");
            eprintln!("SUBSYSTEM==\"usb\", ATTR{{idVendor}}==\"C2AB\", ATTR{{idProduct}}==\"3939\", MODE=\"0666\"");
            eprintln!("macOS: No special permissions needed, but try running without sudo");
            return Err(e);
        }
    };

    println!("Connected to Ferris sweep Raw HID interface!");

    let mode_path = get_vim_mode_path();
    let mut file = match File::open(&mode_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open vim mode file at {:?}", mode_path);
            eprintln!("Error: {}", e);
            eprintln!("Make sure the file exists and contains the current vim mode.");
            eprintln!("You can set a custom path using the VIM_MODE_FILE environment variable.");
            return Err(e.into());
        }
    };
    
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
