use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use display::{DisplayModule, DisplaySection};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use esp_idf_hal::{
    gpio::{PinDriver, Pull},
    prelude::Peripherals,
    sys::{esp_err_to_name, nvs_flash_init, ESP_OK},
};
use esp_idf_svc::sntp::EspSntp;
use http::Http;
use wifi::wifi;

mod display;
mod http;
mod wifi;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default("")]
    sol_rpc: &'static str,
    #[default("")]
    wallet_address: &'static str,
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let app_config = CONFIG;

    let init_result = unsafe { nvs_flash_init() };
    if init_result != ESP_OK {
        unsafe {
            log::error!("Error initializing nvs: {:?}", esp_err_to_name(init_result));
        }
    }

    let peripherals = Peripherals::take().unwrap();

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let mut led_1 = PinDriver::output(peripherals.pins.gpio19).unwrap();
    let mut led_2 = PinDriver::output(peripherals.pins.gpio14).unwrap();
    let mut led_3 = PinDriver::output(peripherals.pins.gpio15).unwrap();
    let mut off_btn = PinDriver::input(peripherals.pins.gpio18).unwrap();
    off_btn.set_pull(Pull::Up).unwrap();

    let mut show_balance_btn = PinDriver::input(peripherals.pins.gpio4).unwrap();
    show_balance_btn.set_pull(Pull::Up).unwrap();

    let mut show_tps_btn = PinDriver::input(peripherals.pins.gpio13).unwrap();
    show_tps_btn.set_pull(Pull::Up).unwrap();

    let mut show_solana_price_btn = PinDriver::input(peripherals.pins.gpio26).unwrap();
    show_solana_price_btn.set_pull(Pull::Up).unwrap();

    let mut show_wallet_qr_code_btn = PinDriver::input(peripherals.pins.gpio27).unwrap();
    show_wallet_qr_code_btn.set_pull(Pull::Up).unwrap();

    let display_module = Arc::new(Mutex::new(DisplayModule::init(
        i2c,
        sda,
        scl,
        &app_config.wallet_address,
    )));

    let display_section = Arc::new(Mutex::new(DisplaySection::Balance));

    let solana_cool_app_text = "Connecting wifi...";

    {
        let mut display = display_module.lock().unwrap();
        display.create_centered_text(solana_cool_app_text, FONT_6X10);
        std::thread::sleep(Duration::from_millis(3000));
        display.create_black_rectangle();
    }

    led_1.set_high().unwrap();

    // initialize wifi
    let _wifi = wifi(
        peripherals.modem,
        &app_config.wifi_ssid,
        app_config.wifi_psk,
    );

    let _sntp = EspSntp::new_default().unwrap();
    let mut http = Http::init(&app_config.sol_rpc).expect("Http module initialization failed");

    let device_ready = "Device Ready";

    {
        let mut display = display_module.lock().unwrap();
        display.create_centered_text(device_ready, FONT_6X10);
        std::thread::sleep(Duration::from_millis(3000));
        display.create_black_rectangle();
    }

    led_1.set_low().unwrap();

    // After device is ready, we're going to create multiple threads to
    // control all separately with the buttons

    let display_clone1 = Arc::clone(&display_module);

    let balance = Arc::new(Mutex::new(0u64));
    let balance_clone_1 = Arc::clone(&balance);
    let _display_section_clone = Arc::clone(&display_section);
    let mut prev_value = 1u64;

    std::thread::spawn(move || {
        const LOOP_DELAY: Duration = Duration::from_millis(150);
        loop {
            led_2.set_high().unwrap();
            match *_display_section_clone.lock().unwrap() {
                DisplaySection::Balance => {
                    let mut display = display_clone1.lock().unwrap();
                    let balance_value = *balance_clone_1.lock().unwrap();
                    if prev_value != balance_value {
                        display.show_balance(balance_value);
                        prev_value = balance_value;
                    }
                }
                DisplaySection::Tps => {
                    let mut display = display_clone1.lock().unwrap();
                    display.show_tps((1, 1));
                }
                DisplaySection::SolPrice => {
                    let mut display = display_clone1.lock().unwrap();
                    display.show_sol_usd_price(15f64);
                }
                DisplaySection::QrCode => {
                    let mut display = display_clone1.lock().unwrap();
                    display.draw_qr_code();
                }
                DisplaySection::ScreenOff => {
                    led_2.set_low().unwrap();
                    let mut display = display_clone1.lock().unwrap();
                    display.draw_image();
                    led_3.set_high().unwrap();
                }
            }
            led_3.set_low().unwrap();
            std::thread::sleep(LOOP_DELAY);
        }
    });

    let display_section_balance = Arc::clone(&display_section);
    std::thread::spawn(move || loop {
        if show_balance_btn.is_low() {
            *display_section_balance.lock().unwrap() = DisplaySection::Balance;
            println!("balance btn pressed",);
        } else {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        std::thread::sleep(Duration::from_millis(5000));
    });

    let display_section_price = Arc::clone(&display_section);
    std::thread::spawn(move || loop {
        if show_solana_price_btn.is_low() {
            *display_section_price.lock().unwrap() = DisplaySection::SolPrice;
            println!("solana price btn pressed",);
        } else {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        std::thread::sleep(Duration::from_millis(5000));
    });

    let display_section_tps = Arc::clone(&display_section);
    std::thread::spawn(move || loop {
        if show_tps_btn.is_low() {
            *display_section_tps.lock().unwrap() = DisplaySection::Tps;
            println!("show tps btn pressed",);
        } else {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        std::thread::sleep(Duration::from_millis(5000));
    });

    let display_section_qr_code = Arc::clone(&display_section);
    std::thread::spawn(move || loop {
        if show_wallet_qr_code_btn.is_low() {
            *display_section_qr_code.lock().unwrap() = DisplaySection::QrCode;
            println!("qr code btn pressed",);
        } else {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        std::thread::sleep(Duration::from_millis(5000));
    });

    let display_section_off = Arc::clone(&display_section);
    std::thread::spawn(move || {
        loop {
            if off_btn.is_low() {
                *display_section_off.lock().unwrap() = DisplaySection::ScreenOff;
                println!("off btn pressed",);
            } else {
                std::thread::sleep(Duration::from_millis(500)); // pulse btn time
                continue;
            }
            std::thread::sleep(Duration::from_millis(5000)); // min time to change the state (On,Off) again
        }
    });

    loop {
        let balance_value = http.get_balance(&app_config.wallet_address).unwrap_or(0);
        let (slot, tps) = http.get_tps().unwrap();
        *balance.lock().unwrap() = balance_value;
        std::thread::sleep(Duration::from_millis(5000));
    }
}
