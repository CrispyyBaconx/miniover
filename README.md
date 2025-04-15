# Miniover

A minimal production-ready Pushover client for Windows.

## Features

- System tray integration
- Windows toast notifications for Pushover messages
- Auto-start on Windows boot option
- Secure storage of credentials
- WebSocket connection for real-time push notifications
- Support for emergency priority messages

## Usage

1. Run the application. It will appear in your system tray.
2. Right-click the tray icon and select "Login".
3. Enter your Pushover account email and password.
4. Once logged in, you'll receive Windows toast notifications for your Pushover messages.
5. You can toggle the "Start on boot" option via the tray menu.

## Requirements

- Windows 10/11
- Pushover account
- Pushover for Desktop license (required within 30 days of activation)

## Building from Source

```
cargo build --release
```

The compiled executable will be located at `target/release/miniover.exe`.

## License

See the LICENSE file for details.
