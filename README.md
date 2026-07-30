Here is the complete, updated **`README.md`** fully aligned with the CLI subcommands (`list` and `monitor`), updated dependencies, and built-in device discovery workflow:

```markdown
# USB Security Guard & Locker (`Molly`)

A lightweight, system-level security daemon and CLI tool written in **Rust** that monitors the USB bus in real time using `libusb` / `rusb`. It inspects connected USB devices against an explicit configuration policy, logs security audit events, and triggers OS-level defense mechanisms (such as screen locking or custom alert handlers) whenever an unauthorized or untrusted USB device is inserted.

---

## Key Features

* **Built-in Device Inspection**: Quickly list connected USB hardware details (VID, PID, Serial, Manufacturer) directly from the CLI.
* **Real-Time Hotplug Monitoring**: Uses event-driven kernel callbacks (`rusb::Hotplug`) rather than high-overhead CPU polling loops.
* **Strict Allowlisting**: Validate devices using a combination of **Vendor ID (VID)**, **Product ID (PID)**, and optional **Serial Number** string descriptors.
* **Initial Bus Enumeration**: Automatically scans and flags existing USB devices connected prior to service startup.
* **Automated Defense Action**: Instantly executes system actions upon detecting unauthorized hardware (e.g., locking workstation sessions on Linux, Windows, or macOS).
* **Configuration via TOML**: Simple, human-readable policy management (`config.toml`).

---

## Architecture Overview


```

```
           ┌───────────────────────────────┐
           │    OS Kernel USB Subsystem    │
           └───────────────┬───────────────┘
                           │ Hotplug Event (Attach / Detach)
                           ▼
           ┌───────────────────────────────┐
           │       rusb Event Loop        │
           └───────────────┬───────────────┘
                           │
                           ▼
           ┌───────────────────────────────┐
           │    UsbGuardHandler Engine     │
           └───────────────┬───────────────┘
                           │
             ┌─────────────┴─────────────┐
             │ Reads Device Descriptor  │
             │ & Serial Number String   │
             └─────────────┬─────────────┘
                           │
                           ▼
         ┌───────────────────────────────────┐
         │ Evaluates against config.toml    │
         └─────────┬─────────────────┬───────┘
                   │                 │
       Matches     │                 │ No Match
      Allowlist    ▼                 ▼
         ┌─────────────────┐ ┌─────────────────────────┐
         │ [ALLOWED] Log   │ │ [ALERT] Trigger Defense │
         │ Safe Device     │ │ (Lock Screen / Audit)   │
         └─────────────────┘ └─────────────────────────┘

```

```

---

## Prerequisites & Dependencies

### System Requirements
* **Rust**: 1.70+ (`cargo`, `rustc`)
* **C Compiler**: Required for building native `libusb` bindings (`gcc` or `clang`).
* **libusb Development Headers**:
  * **Debian / Ubuntu / Kali**: `sudo apt install libusb-1.0-0-dev pkg-config`
  * **Fedora / RHEL**: `sudo dnf install libusb1-devel pkgconf-pkg-config`
  * **macOS**: `brew install libusb pkg-config`
  * **Windows**: Requires `vcpkg` or pre-built `libusb` binaries configured in your environment.

### Cargo Dependencies
```toml
[dependencies]
rusb = "0.9"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
ctrlc = "3.4"
clap = { version = "4.4", features = ["derive"] }

```

---

## Configuration (`config.toml`)

Place `config.toml` in your working directory or pass its location via the `--config` flag when running the `monitor` command.

```toml
# General Security Policy
block_unknown_mass_storage = true
lock_screen_on_unauthorized = false # Set to true to enable automated workstation locking

# Allowlisted Hardware
[[allowed_devices]]
name = "Workstation Mouse"
vendor_id = 0x046D
product_id = 0xC52B

[[allowed_devices]]
name = "Personal Phone (MTP Mode)"
vendor_id = 0x22B8
product_id = 0x2E82
serial_number = "ZX1G223456" # Optional: strictly match hardware serial number

```

> **Finding your Device IDs:**
> You can easily discover VID, PID, and Serial Numbers of currently connected hardware by running the built-in listing command:
> ```bash
> sudo cargo run -- list
> 
> ```
> 
> 

---

## Build & Installation

1. **Clone the repository**:
```bash
git clone [https://github.com/your-username/usb-guard-rs.git](https://github.com/your-username/usb-guard-rs.git)
cd usb-guard-rs

```


2. **Build debug or release binary**:
```bash
cargo build --release

```


3. **Executable location**:
```bash
./target/release/usb-guard-rs

```



---

## Usage & Execution

Because low-level USB bus access requires permission to open hardware handles (`/dev/bus/usb/*` on Linux or Administrator privileges on Windows), the tool **must be run with elevated privileges**.

### 1. List Connected USB Devices

View all currently attached USB devices along with their VID, PID, manufacturer, product, and serial string descriptors:

```bash
sudo cargo run -- list

```

**Example Output:**

```text
VID        PID        Manufacturer         Product              Serial         
---------------------------------------------------------------------------
1d6b       0002       Linux 6.8.0-40-ge... xhci-hcd             0000:00:14.0   
046d       c52b       Logitech             USB Receiver         N/A            
10c4       ea60       Silicon Labs         CP2102 USB to UART   0001           

```

### 2. Run the Monitoring Service

Start the real-time USB hotplug daemon:

```bash
sudo cargo run -- monitor

```

To specify a custom configuration file path:

```bash
sudo cargo run -- monitor --config /etc/usbguard/config.toml

```

**Example Terminal Output:**

```text
USB Security Guard Service started. Monitoring USB bus....

[EVENT] USB Plugged in -> VID: 046d, PID: c52b
[ALLOWED] Safe device attached: 046d:c52b

[EVENT] USB Plugged in -> VID: 13fe, PID: 4200
[ALERT] Unauthorized USB device detected! VID: 13fe, PID: 4200, Serial: Some("070A883C1A12")
[AUDIT] Security event logged for 13fe:4200

[EVENT] USB Disconnected -> VID: 13fe, PID: 4200
^C
USB Security Guard Service Shutting down

```

---

## OS Defense Integration

When `lock_screen_on_unauthorized = true` is set, the service executes native operating system commands upon violation:

* **Linux**: `loginctl lock-session`
* **Windows**: `rundll32.exe user32.dll,LockWorkStation`
* **macOS**: `pmset displaysleepnow`

---

## Running as a Linux System Daemon (`systemd`)

To run this tool automatically on boot as a background security service:

1. Copy the compiled binary to `/usr/local/bin/usb-guard-rs`.
2. Copy your `config.toml` to `/etc/usb-guard/config.toml`.
3. Create `/etc/systemd/system/usb-guard.service`:

```ini
[Unit]
Description=USB Security Guard & Locker Daemon
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/etc/usb-guard
ExecStart=/usr/local/bin/usb-guard-rs monitor --config /etc/usb-guard/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target

```

4. Enable and start the service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now usb-guard

```



```

```