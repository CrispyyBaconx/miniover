# Miniover

A minimal production-ready Pushover client for Windows and Linux.

## Features

- System tray integration
- Desktop notifications for Pushover messages
- Auto-start on boot option (Windows registry / Linux systemd)
- Secure storage of credentials
- WebSocket connection for real-time push notifications
- Support for emergency priority messages

## Usage

1. Run the application. It will appear in your system tray.
2. On first run, enter your Pushover account email and password when prompted.
3. Once logged in, you'll receive desktop notifications for your Pushover messages.
4. Right-click the tray icon for options (toggle autostart, show logs, about, logout, quit).

## Requirements

- **Windows 10/11** or **Linux** (tested on Arch Linux)
- Pushover account
- Pushover for Desktop license (required within 30 days of activation)

### Linux Dependencies (Arch Linux)

```bash
# Required for GTK4 login dialog and system tray support
sudo pacman -S gtk4 libappindicator-gtk3

# Required for notifications (usually pre-installed)
sudo pacman -S libnotify
```

For other distros, install the equivalent packages for `gtk4`, `libappindicator`, and `libnotify`.

## Building from Source

```bash
cargo build --release
```

The compiled executable will be located at:
- **Windows:** `target/release/miniover.exe`
- **Linux:** `target/release/miniover`

## Installation

### Windows

Simply run `miniover.exe`. Use the tray menu to enable "Start on boot" if desired.

### Linux

1. Build and install the binary:
```bash
cargo build --release
cargo install --path .
```

2. Enable autostart with systemd (user service):
```bash
# Copy the service file
mkdir -p ~/.config/systemd/user
cp miniover.service ~/.config/systemd/user/

# Enable and start the service
systemctl --user daemon-reload
systemctl --user enable miniover.service
systemctl --user start miniover.service

# Check status
systemctl --user status miniover.service
```

Alternatively, use the tray menu's "Start on boot" option which uses `auto-launch`.

## Configuration

Configuration is stored in:
- **Windows:** `%APPDATA%\miniover\config.json`
- **Linux:** `~/.config/miniover/config.json`

Logs are stored in:
- **Windows:** `%APPDATA%\miniover\logs\`
- **Linux:** `~/.local/share/miniover/logs/`

## License

See the LICENSE file for details.
