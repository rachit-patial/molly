use clap::{Parser, Subcommand};
use rusb::{Context, Device, DeviceDescriptor, Hotplug, Registration, UsbContext};
use serde::Deserialize;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "usbguard")]
#[command(about = "A CLI tool and background service to monitor and secure USB devices", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all currently connected USB devices
    List,
    /// Run the background USB monitoring and security service
    Monitor {
        /// Path to the configuration file
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
}

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

            println!("[EVENT] USB Plugged in -> VID: {:04x}, PID: {:04x}", vid, pid);

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

fn list_devices() -> rusb::Result<()> {
    println!("{:<10} {:<10} {:<20} {:<20} {:<15}", "VID", "PID", "Manufacturer", "Product", "Serial");
    println!("{}", "-".repeat(75));

    for device in rusb::devices()?.iter() {
        let descriptor = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        let handle = device.open();
        let timeout = Duration::from_millis(200);

        let (mfg, prod, serial) = match handle {
            Ok(ref h) => (
                h.read_manufacturer_string_ascii(&descriptor).unwrap_or_else(|_| "Unknown".into()),
                h.read_product_string_ascii(&descriptor).unwrap_or_else(|_| "Unknown".into()),
                h.read_serial_number_string_ascii(&descriptor).unwrap_or_else(|_| "N/A".into()),
            ),
            Err(_) => ("Access Denied".into(), "Access Denied".into(), "N/A".into()),
        };

        println!(
            "{:04x}       {:04x}       {:<20} {:<20} {:<15}",
            descriptor.vendor_id(),
            descriptor.product_id(),
            truncate(&mfg, 18),
            truncate(&prod, 18),
            truncate(&serial, 15)
        );
    }

    Ok(())
}

fn run_monitor(config_path: &str) -> rusb::Result<()> {
    let config_raw = fs::read_to_string(config_path).unwrap_or_else(|_| {
        eprintln!("Warning: {} not found, defaulting to strict mode.", config_path);
        String::from("block_unknown_mass_storage = true\nlock_screen_on_unauthorized=true\nallowed_devices = []")
    });

    let config: Config = toml::from_str(&config_raw).expect("Invalid config format");

    if !rusb::has_hotplug() {
        eprintln!("Error: OS/libusb platform does not support USB hotplug events.");
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

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

fn main() -> rusb::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => list_devices()?,
        Commands::Monitor { config } => run_monitor(&config)?,
    }

    Ok(())
}