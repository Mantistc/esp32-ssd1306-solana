# Esp32-ssd1306-solana

Show Solana real-time data in a little ssd1306 display using the microcontroller esp32 to manage wifi, http request and more.

---

## **Features**
- Wifi/bluetooth
- Http/Https request
- Multiple GPIOs pin connections

---

## **Getting Started**

### **Requirements**
1. **Rust** (latest stable version).
2. An esp32 microcontroller
3. A ssd1306 mini display
4. Some jumper wires

### **Installation**
1. Clone this repository:
   ```bash
   git clone https://github.com/Mantistc/esp32-ssd1306-solana
   cd esp32-ssd1306-solana
   ```
2. Create a `cfg.tolm` file like the `cfg.tolm.example` and fill it with your stuff
3. Connect your ssd1306 display to your esp32 and connect the esp32 to your computer
4. Build the application:
   ```bash
   cargo build --release
   ```
5. Run the application:
   ```bash
   cargo run
   ```

Enjoy your cool mini display showing you how solana is pumping
---

<p align="center">
  Made with ❤️ by <a href="https://twitter.com/lich01_" target="_blank">@lich.sol</a>
</p>
