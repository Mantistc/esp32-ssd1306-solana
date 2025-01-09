use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use display::DisplayModule;
use embedded_graphics::mono_font::ascii::{FONT_4X6, FONT_6X10};
use esp_idf_hal::{
    gpio::{PinDriver, Pull},
    prelude::Peripherals,
    sys::{esp_err_to_name, nvs_flash_init, ESP_OK},
};
use http::{Http, LAMPORTS_PER_SOL};
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
            let max_width_size = 128;
            let label = "Sol Balance:";
            let label_x_c = (max_width_size - label.len() * 6) / 2;
            let label_y_c = 16;

            let wallet_balance = http
                .get_balance("5KgfWjGePnbFgDAuCqxB5oymuFxQskvCtrw6eYfDa7fj")
                .unwrap_or(0);
            let readable_result = wallet_balance as f32 / LAMPORTS_PER_SOL as f32;

            led_2.set_high().unwrap();

            let formatted = format!("{:.2}", readable_result);
            let value_x_c = (max_width_size - formatted.len() * 6) / 2;
            let value_x_y = 33;

            display_module.create_text(&label, label_x_c as u8, label_y_c, FONT_6X10);
            display_module.create_text(&formatted, value_x_c as u8, value_x_y, FONT_6X10);

            std::thread::sleep(Duration::from_millis(3000));

            let (slot, tps) = http.get_tps().unwrap_or_default();

            display_module.create_black_rectangle();

            let height_constant = 6 + 5;
            let font_width_4x = 4;
            let font_width_6x = 6;

            let slot_label = "Slot:";
            let slot_label_x_c = (max_width_size - slot_label.len() * font_width_4x) / 2;
            let slot_label_y_c = 8;

            let slot_value_x_c = (max_width_size - slot.to_string().len() * font_width_6x) / 2;
            let slot_value_y_c = slot_label_y_c + height_constant;

            let tps_label = "TPS:";
            let tps_label_x_c = (max_width_size - tps_label.len() * font_width_4x) / 2;
            let tps_label_y_c = slot_value_y_c + height_constant + 6;

            let tps_value_x_c = (max_width_size - tps.to_string().len() * font_width_6x) / 2;
            let tps_value_y_c = tps_label_y_c + height_constant;

            //slot
            display_module.create_text(&slot_label, slot_label_x_c as u8, slot_label_y_c, FONT_4X6);
            display_module.create_text(
                &slot.to_string(),
                slot_value_x_c as u8,
                slot_value_y_c,
                FONT_6X10,
            );

            // tps
            display_module.create_text(&tps_label, tps_label_x_c as u8, tps_label_y_c, FONT_4X6);
            display_module.create_text(
                &tps.to_string(),
                tps_value_x_c as u8,
                tps_value_y_c,
                FONT_6X10,
            );

            std::thread::sleep(Duration::from_millis(3000));

            let sol_price_label = "Sol USD Price:";
            let sol_price_label_x_c = (max_width_size - sol_price_label.len() * 6) / 2;
            let sol_price_label_y_c = 16;

            let sol_price = http.get_solana_price().unwrap_or_default();

            display_module.create_black_rectangle();

            let sol_price_formatted = format!("{:.2}", sol_price);
            let sol_price_x_c = (max_width_size - sol_price_formatted.len() * 6) / 2;
            let sol_price_x_y = 33;

            display_module.create_text(
                &sol_price_label,
                sol_price_label_x_c as u8,
                sol_price_label_y_c,
                FONT_6X10,
            );
            display_module.create_text(
                &sol_price_formatted,
                sol_price_x_c as u8,
                sol_price_x_y,
                FONT_6X10,
            );

            std::thread::sleep(Duration::from_millis(1500));
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
