use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use display::DisplayModule;
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

    let is_on = Arc::new(AtomicBool::new(true));
    let is_on_clone = Arc::clone(&is_on);

    let display_module = Arc::new(Mutex::new(DisplayModule::init(
        i2c,
        sda,
        scl,
        &app_config.wallet_address,
    )));

    std::thread::spawn(move || loop {
        if off_btn.is_low() {
            is_on_clone.store(!is_on_clone.load(Ordering::SeqCst), Ordering::SeqCst);
            println!(
                "Button toggled. is_on: {}",
                is_on_clone.load(Ordering::SeqCst)
            );
        } else {
            std::thread::sleep(Duration::from_millis(500)); // pulse btn time
            continue;
        }
        std::thread::sleep(Duration::from_millis(5000)); // min time to change the state (On,Off) again
    });


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

    let device_ready = "Device Ready";

    {
        let mut display = display_module.lock().unwrap();
        display.create_centered_text(device_ready, FONT_6X10);
        std::thread::sleep(Duration::from_millis(3000));
        display.create_black_rectangle();
    }

    led_1.set_low().unwrap();


    // After device is ready, we're going to separate into threads
    // with that, we can control all 

    let display_clone1 = Arc::clone(&display_module);
    let is_on_clone = Arc::clone(&is_on);
    let http = Arc::new(Mutex::new(
        Http::init(&app_config.sol_rpc).expect("Http module initialization failed"),
    ));
    let http_clone = Arc::clone(&http);

    std::thread::spawn(move || loop {
        let mut http = http_clone.lock().unwrap();
        let mut display = display_clone1.lock().unwrap();
        let show_data = is_on_clone.load(Ordering::SeqCst);
        if show_data {
            display.show_balance(&mut http);
            std::thread::sleep(Duration::from_millis(50));
        }
    });

    let is_on_clone = Arc::clone(&is_on);
    let display_clone2 = Arc::clone(&display_module);

    std::thread::spawn(move || loop {
        let show_data = is_on_clone.load(Ordering::SeqCst);
        let mut display = display_clone2.lock().unwrap();
        if !show_data {
            display.create_black_rectangle();
            println!("Device Off");
            display.draw_image();
            led_2.set_low().unwrap();
            led_3.set_high().unwrap();
            std::thread::sleep(Duration::from_millis(50));
        }
    });

    loop {
        std::thread::sleep(Duration::from_millis(500));
    }
}
