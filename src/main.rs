use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use display::{DisplayModule, DisplaySection, DisplayValues};
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

    let display_values = Arc::new(Mutex::new(DisplayValues::default()));
    let display_values_clone = Arc::clone(&display_values);
    let display_section = Arc::new(Mutex::new(DisplaySection::Balance));
    let display_section_clone = Arc::clone(&display_section);
    let mut prev_values = DisplayValues {
        tps_values: (1, 1),
        balance: 1,
        sol_price: 1f64,
        date_values: ("...".to_string(), "...".to_string()),
    };

    const LOOP_DELAY: Duration = Duration::from_millis(150);

    std::thread::spawn(move || loop {
        let section = display_section_clone.lock().unwrap();
        let values = display_values_clone.lock().unwrap();
        let mut display = display_clone1.lock().unwrap();

        if let DisplaySection::Balance = *section {
            if prev_values.balance != values.balance {
                display.show_balance(values.balance);
                prev_values.balance = values.balance;
            }
        } else {
            prev_values.balance = u64::MAX;
        }

        if let DisplaySection::Tps = *section {
            if values.tps_values.0 != prev_values.tps_values.0
                || values.tps_values.1 != prev_values.tps_values.1
            {
                display.show_tps(values.tps_values);
                prev_values.tps_values = values.tps_values;
            }
        } else {
            prev_values.tps_values = (0, 0);
        }

        if let DisplaySection::SolPrice = *section {
            if values.sol_price != prev_values.sol_price {
                display.show_sol_usd_price(values.sol_price);
                prev_values.sol_price = values.sol_price;
            }
        } else {
            prev_values.sol_price = 0.0;
        }

        match *section {
            DisplaySection::Tps | DisplaySection::SolPrice | DisplaySection::Balance => {
                led_2.set_high().unwrap();
                led_3.set_low().unwrap();
            }
            DisplaySection::QrCode => {
                display.draw_qr_code();
            }
            DisplaySection::ScreenOff => {
                led_2.set_low().unwrap();
                display.draw_image();
                led_3.set_high().unwrap();
            }
        }

        if prev_values.date_values != values.date_values
            && *section != DisplaySection::QrCode
            && *section != DisplaySection::ScreenOff
        {
            let (date, time) = &values.date_values;
            display.draw_time((&date, &time))
        };

        drop(values);
        std::thread::sleep(LOOP_DELAY);
    });

    let display_section_btns = Arc::clone(&display_section);
    std::thread::spawn(move || loop {
        let new_section = if show_balance_btn.is_low() {
            println!("balance btn pressed");
            Some(DisplaySection::Balance)
        } else if show_solana_price_btn.is_low() {
            println!("show_solana_price_btn pressed");
            Some(DisplaySection::SolPrice)
        } else if show_wallet_qr_code_btn.is_low() {
            println!("show_wallet_qr_code_btn pressed");
            Some(DisplaySection::QrCode)
        } else if show_tps_btn.is_low() {
            println!("show_tps_btn pressed");
            Some(DisplaySection::Tps)
        } else if off_btn.is_low() {
            println!("off_btn pressed");
            Some(DisplaySection::ScreenOff)
        } else {
            std::thread::sleep(Duration::from_millis(500));
            None
        };
        if let Some(section) = new_section {
            *display_section_btns.lock().unwrap() = section;
            std::thread::sleep(Duration::from_millis(1500));
        }
    });

    loop {
        let time = http.get_time().unwrap();
        let balance_value = http.get_balance(&app_config.wallet_address).unwrap_or(0);
        let tps_values = http.get_tps().unwrap();
        let sol_price = http.get_solana_price().unwrap();
        std::thread::sleep(Duration::from_millis(5000));
        {
            let mut display_values = display_values.lock().unwrap();
            display_values.date_values = time;
            display_values.balance = balance_value;
            display_values.tps_values = tps_values;
            display_values.sol_price = sol_price;
        }
    }
}
