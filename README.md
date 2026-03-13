# airpods-bridge

A lightweight Rust utility that streams AirPods battery levels and ANC status as JSON. Primarily built for custom status bars or local automation scripts.

it only test on airpods pro 2 (lightning)

## Features
* **Automatic Discovery**: Automatically scans for paired AirPods if no MAC address is provided.
* **JSON Output**: Provides real-time status including Left/Right/Case battery levels and the current Noise Control mode.

## Still Working
* **ANC Control**: Includes a Unix Socket listener at `/tmp/airpods.sock` to switch modes externally.

## Support
* **Operating System**: Linux only (Arch, Ubuntu, Fedora, etc.)
* **Bluetooth Stack**: BlueZ 5.x

## Usage

Build and run the binary:
```bash
cargo build --release
./target/release/airpods-bridge
```

To target with MAC address 
```bash
./target/release/airpods-bridge AA:BB:CC:DD:EE:FF
```

JSON Format
```
{
  "device_name": "AirPods Pro",
  "model_name": "AirPods Pro (Gen *)"
  "anc_mode": "Noise Cancellation",
  "connected": true,
  "left": { "level": 90, "charging": false },
  "right": { "level": 90, "charging": false },
  "case": { "level": 100, "charging": true }
}
```

# Cycling ANC Modes

Once the bridge is running, send the cycle command to the socket using nc:
```bash
echo "cycle" | nc -U /tmp/airpods.sock
```

