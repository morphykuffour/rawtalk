extern crate hidapi;

use hidapi::HidApi;

fn main() {
    let api = hidapi::HidApi::new().unwrap();
    
    // First, let's find your QMK keyboard
    // println!("Looking for QMK keyboard...");
    // for device in api.device_list() {
    //     println!("Found device: VID: {:04x}, PID: {:04x}, Manufacturer: {:?}, Product: {:?}",
    //         device.vendor_id(),
    //         device.product_id(),
    //         device.manufacturer_string(),
    //         device.product_string());
    // }

    // You'll need to replace these with your keyboard's actual VID/PID
    // Common QMK VID is 0xFEED or manufacturer specific (e.g., 0x445A for Keychron)
    let (vid, pid) = (0xFEED, 0x0000); // Replace with your keyboard's VID/PID
    
    match api.open(vid, pid) {
        Ok(device) => {
            println!("Successfully connected to QMK keyboard!");
            
            // Example: Send layer change command
            // The exact protocol depends on your QMK configuration
            // Typically, the first byte is the report ID (0x00)
            let layer_command = [0x00, 0x01]; // Replace with your keyboard's protocol
            match device.write(&layer_command) {
                Ok(res) => println!("Sent layer change command: {:?} bytes written", res),
                Err(e) => eprintln!("Failed to send command: {}", e),
            }
        },
        Err(e) => eprintln!("Failed to connect to keyboard: {}", e),
    }
}
