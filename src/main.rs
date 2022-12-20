extern crate hidapi;

use hidapi::HidApi;

fn main() {
    println!("Printing all available hid devices:");

    match HidApi::new() {
        Ok(api) => {
            for device in api.device_list() {
                println!("{:04x}:{:04x}", device.vendor_id(), device.product_id());
                // println!("{:04x}", device.product_string_raw());
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
        },
    }
}

// extern crate hidapi;
//
// use hidapi::HidApi;
//
// fn main() {
//     let api = hidapi::HidApi::new().unwrap();
//     // Print out information about all connected devices
//     for device in api.device_list() {
//         println!("{:#?}", device);
//     }
//
//     // Connect to device using its VID and PID
//     let (VID, PID) = (0x0123, 0x3456);
//     let device = api.open(VID, PID).unwrap();
//
//     // Read data from device
//     let mut buf = [0u8; 8];
//     let res = device.read(&mut buf[..]).unwrap();
//     println!("Read: {:?}", &buf[..res]);
//
//     // Write data to device
//     let buf = [0u8, 1, 2, 3, 4];
//     let res = device.write(&buf).unwrap();
//     println!("Wrote: {:?} byte(s)", res);
// }
