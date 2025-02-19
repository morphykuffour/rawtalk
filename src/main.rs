extern crate hidapi;

use hidapi::HidApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    
    // Ferris sweep VID/PID values
    let (VID, PID) = (0xC2AB, 0x3939);
    
    match api.open(VID, PID) {
        Ok(device) => {
            println!("Successfully connected to Ferris sweep!");
            
            // Example: Send layer change command
            // The exact protocol depends on your QMK configuration
            let layer_command = [0x00, 0x01]; // Replace with your keyboard's protocol
            match device.write(&layer_command) {
                Ok(res) => println!("Sent layer change command: {:?} bytes written", res),
                Err(e) => eprintln!("Failed to send command: {}", e),
            }
        },
        Err(e) => eprintln!("Failed to connect to keyboard: {}", e),
    }

    Ok(())
}
