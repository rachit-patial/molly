use rusb::{Context, Device, DeviceDescriptor, Hotplug, Registration, UsbContext};
use serde::Deserialize;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Config {
    block_unknown_mass_storage: bool,
    lock_screen_on_unauthorized: bool,
    allowed_devices: Vec<AllowedDevice>,
}

#[derive(Debug, Deserialize)]
struct AllowedDevice {
    name: String,
    vendor_id: u16,
    product_id: u16,
    serial_number: Option<String>,
}

struct UsbGuardHandler {
    config: Config,
}

impl UsbGuardHandler {
    fn inspect_and_enforce(&self, device: &Device<Context>, descriptor: DeviceDescriptor) {
        let vid = descriptor.vendor_id();
        let pid = descriptor.product_id();

        let handle = device.open();
        let serial = match &handle {
            Ok(h) => h.read_serial_number_string_ascii(&descriptor).ok(),
            Err(_) => None,
        };

        let is_allowed = self.config.allowed_devices.iter().any(|allowed| {
            if allowed.vendor_id != vid || allowed.product_id != pid {
                return false;
            }
            if let Some(ref expected_serial) = allowed.serial_number {
                return serial.as_ref() == Some(expected_serial);
            }
            true
        });

        if is_allowed {
            println!("[ALLOWED] Safe device attached: {:04x}:{:04x}", vid, pid);
        } else {
            eprintln!(
                "[ALERT] Unauthorized USB device detected! VID: {:04x}, PID: {:04x}, Seria: {:?}",
                vid, pid, serial
            );
            self.trigger_defense_action(vid, pid);
        }
    }

    fn trigger_defense_action(&self, vid: u16, pid: u16) {
        eprintln!("[AUDIT] Security event logged for {:04x}:{:04x}", vid, pid);

        if self.config.lock_screen_on_unauthorized {
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("loginctl")
                .arg("lock-session")
                .status();

            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("rundll32.exe")
                .args(["user32.dll,LockWorkStation"])
                .status();

            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("pmset")
                .args(["displaysleepnow"])
                .status();
        }
    }
}

impl<T: UsbContext> Hotplug<T> for UsbGuardHandler {
    fn device_arrived(&mut self, device: Device<T>) {
        if let Ok(descriptor) = device.device_descriptor() {
            let vid = descriptor.vendor_id();
            let pid = descriptor.product_id();

            println!(
                "[EVENT] USB Plugged in -> VID: {:04x}, PID: {:04x}",
                vid, pid
            );

            let is_known = self
                .config
                .allowed_devices
                .iter()
                .any(|d| d.vendor_id == vid && d.product_id == pid);
            if !is_known {
                self.trigger_defense_action(vid, pid);
            }
        }
    }

    fn device_left(&mut self, device: Device<T>) {
        if let Ok(descriptor) = device.device_descriptor() {
            println!(
                "[EVENT] USB Disconnected -> VID: {:04x}, PID: {:04x}",
                descriptor.vendor_id(),
                descriptor.product_id()
            );
        }
    }
}

fn main() -> rusb::Result<()> {
    let config_raw = fs::read_to_string("config.toml").unwrap_or_else(|_| {
        eprintln!("Warning: config.toml not found, defaulting to strict mode.");
        String::from("block_unknown_mass_storage = true\nlock_screen_on_unauthorized=true\nallowed_devices = []")
    });

    let config: Config = toml::from_str(&config_raw).expect("Invalid config.toml format");

    if !rusb::has_hotplug() {
        eprintln!("Error: OS/libsub platform does not support USB hotplug events.");
        return Ok(());
    }

    let context = Context::new()?;
    let handler = UsbGuardHandler { config };

    let _registration: Option<Registration<Context>> = Some(
        rusb::HotplugBuilder::new()
            .enumerate(true)
            .register(&context, Box::new(handler))?,
    );

    println!("USB Security Guard Service started. Monitoring USB bus....");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let _ = ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    });

    while running.load(Ordering::SeqCst) {
        context.handle_events(Some(Duration::from_secs(1)))?;
    }

    println!("USB Security Guard Service Shutting down");

    Ok(())
}
