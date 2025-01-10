use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
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
    let mut button = PinDriver::input(peripherals.pins.gpio18).unwrap();
    button.set_pull(Pull::Up).unwrap();

    let is_on = Arc::new(AtomicBool::new(true));
    let is_on_clone = Arc::clone(&is_on);
    let mut display_module = DisplayModule::init(i2c, sda, scl);

    std::thread::spawn(move || loop {
        if button.is_low() {
            is_on_clone.store(!is_on_clone.load(Ordering::SeqCst), Ordering::SeqCst);
            println!(
                "Button toggled. is_on: {}",
                is_on_clone.load(Ordering::SeqCst)
            );
        } else {
            std::thread::sleep(Duration::from_millis(500)); // pulse btn time
            continue;
        }
        std::thread::sleep(Duration::from_millis(10000)); // min time to change the state (On,Off) again
    });

    // initialize display

    let solana_cool_app_text = "Connecting wifi...";

    led_1.set_high().unwrap();

    display_module.create_centered_text(&solana_cool_app_text, FONT_6X10);

    // initialize wifi
    let _wifi = wifi(
        peripherals.modem,
        &app_config.wifi_ssid,
        app_config.wifi_psk,
    );

    let mut http = Http::init(&app_config.sol_rpc).expect("Http module initialization failed");

    display_module.create_black_rectangle();

    let device_ready = "Device Ready";

    led_1.set_high().unwrap();

    display_module.create_centered_text(&device_ready, FONT_6X10);

    led_1.set_low().unwrap();
    let mut previous_state = true;
    loop {
        let show_data = is_on.load(Ordering::SeqCst);
        if show_data {
            display_module.create_black_rectangle();
            if !previous_state {
                previous_state = true;
            }
            led_3.set_low().unwrap();
            display_module.perpetual_data(&mut http);
        } else if !show_data && previous_state {
            display_module.create_black_rectangle();
            println!("Device Off");
            display_module.draw_image();
            led_2.set_low().unwrap();
            led_3.set_high().unwrap();
            previous_state = false;
            std::thread::sleep(Duration::from_millis(3000));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
