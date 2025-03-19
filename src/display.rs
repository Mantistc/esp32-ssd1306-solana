use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{
        ascii::{FONT_4X6, FONT_6X10},
        MonoFont, MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::{Point, Primitive, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
    Drawable,
};
use esp_idf_hal::{
    gpio::{Gpio21, Gpio22},
    i2c::{I2cConfig, I2cDriver, I2C0},
    units::Hertz,
};
use log::info;
use qrcodegen::{QrCode, QrCodeEcc};
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::{DisplayRotation, I2CInterface},
    size::DisplaySize128x64,
    I2CDisplayInterface, Ssd1306,
};
use std::time::Duration;

use crate::http::{Http, LAMPORTS_PER_SOL};

pub struct DisplayModule {
    pub display: Ssd1306<
        I2CInterface<I2cDriver<'static>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
    pub wallet_address: String,
}
pub const MAX_WIDTH_SIZE: usize = 128;

impl DisplayModule {
    pub fn init(i2c: I2C0, sda: Gpio21, scl: Gpio22, wallet_address: &str) -> Self {
        let mut i2c =
            I2cDriver::new(i2c, sda, scl, &I2cConfig::new().baudrate(Hertz(400))).unwrap();

        for address in 0x00..=0x7F {
            if i2c.write(address, &[], 5000).is_ok() {
                info!("Found device at address: 0x{:02X}", address);
            }
        }
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        let on = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();

        match display.init() {
            Ok(value) => {
                info!("init success");
                value
            }
            Err(err) => {
                info!("Error: {:?}", err);
                panic!("Error on init: {:?}", err);
            }
        };

        Rectangle::new(Point::new(0, 0), Size::new(127, 63))
            .into_styled(on)
            .draw(&mut display)
            .unwrap();
        Self {
            display,
            wallet_address: wallet_address.to_string(),
        }
    }

    pub fn create_centered_text(&mut self, text: &str, font: MonoFont) {
        let text_width = text.len() as u8 * 6;
        let text_height = 10u8;

        let x = (128u8 - text_width) / 2;
        let y = (64u8 - text_height) / 2;

        self.create_text(text, x, y, font);
    }

    pub fn create_text(&mut self, text: &str, x_c: u8, y_c: u8, font: MonoFont) {
        let text_style = MonoTextStyleBuilder::new()
            .font(&font)
            .text_color(BinaryColor::On)
            .build();

        let display = &mut self.display;
        Text::with_baseline(
            text,
            Point::new(x_c.into(), y_c.into()),
            text_style,
            Baseline::Top,
        )
        .draw(display)
        .unwrap();
        display.flush().unwrap();
    }

    pub fn create_black_rectangle(&mut self) {
        let display = &mut self.display;
        let on = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .fill_color(BinaryColor::Off)
            .build();

        Rectangle::new(Point::new(0, 0), Size::new(127, 63))
            .into_styled(on)
            .draw(display)
            .unwrap();
    }

    pub fn draw_image(&mut self) {
        self.create_black_rectangle();
        let display = &mut self.display;
        let size = 32i32;
        let raw: ImageRaw<BinaryColor> =
            ImageRaw::new(include_bytes!("../sol_logo.raw"), size as u32);
        let im = Image::new(&raw, Point::new((128 - size) / 2, (64 - size) / 2));
        im.draw(display).unwrap();
        display.flush().unwrap();
    }

    pub fn draw_qr_code(&mut self) {
        self.create_black_rectangle();
        let display = &mut self.display;
        let qr = QrCode::encode_text(&self.wallet_address, QrCodeEcc::Low).unwrap();
        let qr_size = qr.size();

        let max_width = 128;
        let max_height = 64;
        let padding_y = 6;

        let available_height = max_height - (padding_y * 2);
        let scale = 2;

        // scale the QR to be able to scan it
        let qr_width = qr_size * scale;
        let qr_height = qr_size * scale;

        let offset_x = (max_width - qr_width) / 2;
        let offset_y = ((available_height - qr_height) / 2) + padding_y;

        for y in 0..qr_size {
            for x in 0..qr_size {
                // this condition determines whether we need to draw a pixel or not.
                if qr.get_module(x, y) {
                    Rectangle::new(
                        Point::new(offset_x + x * scale, offset_y + y * scale),
                        Size::new(scale as u32, scale as u32),
                    )
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)
                    .unwrap();
                }
            }
        }
        display.flush().unwrap(); // write the data
    }

    pub fn draw_time(&mut self, http: &mut Http) {
        self.create_black_rectangle();
        let (time, date) = http.utc_offset_time().unwrap_or_default();
        let x = 5;
        let y = 64 - 9;
        self.create_text(&date, x as u8, y, FONT_4X6);
        let x_time = 128 - (time.len() * 4) - 5;
        self.create_text(&time, x_time as u8, y, FONT_4X6);
    }

    pub fn show_balance(&mut self, balance: u64) {
        self.create_black_rectangle();
        let label = "Sol Balance:";
        let label_x_c = (MAX_WIDTH_SIZE - label.len() * 6) / 2;
        let label_y_c = 16;

        let readable_result = balance as f32 / LAMPORTS_PER_SOL as f32;

        let formatted = format!("{:.2}", readable_result);
        let value_x_c = (MAX_WIDTH_SIZE - formatted.len() * 6) / 2;
        let value_x_y = 33;

        self.create_text(&label, label_x_c as u8, label_y_c, FONT_6X10);
        self.create_text(&formatted, value_x_c as u8, value_x_y, FONT_6X10);
        std::thread::sleep(Duration::from_millis(1500));
    }

    pub fn show_tps(&mut self, http: &mut Http) {
        self.create_black_rectangle();
        let (slot, tps) = http.get_tps().unwrap_or_default();
        let height_constant = 6 + 5;
        let font_width_4x = 4;
        let font_width_6x = 6;

        let slot_label = "Slot:";
        let slot_label_x_c = (MAX_WIDTH_SIZE - slot_label.len() * font_width_4x) / 2;
        let slot_label_y_c = 8;

        let slot_value_x_c = (MAX_WIDTH_SIZE - slot.to_string().len() * font_width_6x) / 2;
        let slot_value_y_c = slot_label_y_c + height_constant;

        let tps_label = "TPS:";
        let tps_label_x_c = (MAX_WIDTH_SIZE - tps_label.len() * font_width_4x) / 2;
        let tps_label_y_c = slot_value_y_c + height_constant + 6;

        let tps_value_x_c = (MAX_WIDTH_SIZE - tps.to_string().len() * font_width_6x) / 2;
        let tps_value_y_c = tps_label_y_c + height_constant;

        //slot
        self.create_text(&slot_label, slot_label_x_c as u8, slot_label_y_c, FONT_4X6);
        self.create_text(
            &slot.to_string(),
            slot_value_x_c as u8,
            slot_value_y_c,
            FONT_6X10,
        );
        // tps
        self.create_text(&tps_label, tps_label_x_c as u8, tps_label_y_c, FONT_4X6);
        self.create_text(
            &tps.to_string(),
            tps_value_x_c as u8,
            tps_value_y_c,
            FONT_6X10,
        );
        std::thread::sleep(Duration::from_millis(1500));
    }

    pub fn show_sol_usd_price(&mut self, http: &mut Http) {
        self.create_black_rectangle();
        let sol_price_label = "Sol USD Price:";
        let sol_price_label_x_c = (MAX_WIDTH_SIZE - sol_price_label.len() * 6) / 2;
        let sol_price_label_y_c = 16;
        let sol_price = http.get_solana_price().unwrap_or_default();
        let sol_price_formatted = format!("{:.2}", sol_price);
        let sol_price_x_c = (MAX_WIDTH_SIZE - sol_price_formatted.len() * 6) / 2;
        let sol_price_x_y = 33;

        self.create_text(
            &sol_price_label,
            sol_price_label_x_c as u8,
            sol_price_label_y_c,
            FONT_6X10,
        );
        self.create_text(
            &sol_price_formatted,
            sol_price_x_c as u8,
            sol_price_x_y,
            FONT_6X10,
        );

        std::thread::sleep(Duration::from_millis(1500));
    }
}
