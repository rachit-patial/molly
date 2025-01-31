
use rusb::{Context, Device, UsbContext};

fn main() {
    match is_my_phone_connected() {
        Ok(_) => println!("Usb scan complete."),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn list_usb_devices() -> rusb::Result<()> {
    let context = Context::new()?;
    
    let devices = context.devices()?;
    println!("Found {} USB devices", devices.len());

    for device in devices.iter() {
        let descriptor = device.device_descriptor()?;
        println!(
            "Devices: Vendor ID: {:04X}, Product ID: {:04x}",
            descriptor.vendor_id(),
            descriptor.product_id()
        )
    }

    Ok(())
}

//only for motrolla now
fn is_my_phone_connected() -> rusb::Result<()> {
    let context = Context::new()?;

    let devices = context.devices()?;
    for device in devices.iter() {
        let descriptor = device.device_descriptor()?;
        match descriptor.vendor_id() {
            0x22B8 => {
                println!("Detected your phone");
                break;
            },
            _ => print!(".")
        }
    };

    Ok(())
}