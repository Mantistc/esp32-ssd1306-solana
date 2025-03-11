# Esp32-ssd1306-solana

Show Solana real-time data on a **SSD1306** mini display using an **ESP32** microcontroller.  
The ESP32 handles **Wi-Fi**, **HTTP/HTTPS requests**, and **GPIO connections** to interact with the display.

---

## **Features**
- Wi-Fi & Bluetooth support
- HTTP/HTTPS requests handling
- Multiple GPIO pin connections

---

## **Getting Started**

### **Requirements**
1. **Rust** (latest stable version)
2. **ESP32** microcontroller
3. **SSD1306** mini display
4. **Jumper wires**
5. **USB cable** to connect ESP32 to your computer

---

### **Installation**

#### **1) Install `espup`**
```bash
cargo install espup
```

#### **2) Install Necessary Toolchains**
```bash
espup install
```

#### **3) Install Espressif toolchain**
```bash
cargo install cargo-espflash espflash ldproxy
```

#### **4) Clone this repository**
```bash
git clone https://github.com/Mantistc/esp32-ssd1306-solana
cd esp32-ssd1306-solana
```

#### **5) Create your configuration file**
Create a `cfg.toml` file based on the example file:
```bash
cp cfg.toml.example cfg.toml
```
Then, edit it and add your custom settings.

#### **6) Connect your hardware**
- Connect your **ESP32** to your computer via USB.
- Wire the **SSD1306** display to the correct ESP32 pins.

#### **7) Build the application**
```bash
cargo build --release
```

#### **8) Flash the firmware to the ESP32**
```bash
cargo espflash flash --monitor
```
or

```bash
cargo run --release
```
---

## **Enjoy!**
Now your cool mini display will show you **real-time Solana data**!  

---

<p align="center">
  Made with ❤️ by <a href="https://twitter.com/lich01_" target="_blank">@lich.sol</a>
</p>
