use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoFont, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::{Point, Primitive, Size},
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
    Drawable,
};
use esp_idf_hal::{
    gpio::{Gpio21, Gpio22},
    i2c::{I2cConfig, I2cDriver, I2C0},
    units::Hertz,
};
use log::info;
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::{DisplayRotation, I2CInterface},
    size::DisplaySize128x64,
    I2CDisplayInterface, Ssd1306,
};

pub struct DisplayModule {
    pub display: Ssd1306<
        I2CInterface<I2cDriver<'static>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
}

impl DisplayModule {
    pub fn init(i2c: I2C0, sda: Gpio21, scl: Gpio22) -> Self {
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
        Self { display }
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
        let display = &mut self.display;
        let size = 32i32;
        let raw: ImageRaw<BinaryColor> =
            ImageRaw::new(include_bytes!("../sol_logo.raw"), size as u32);
        let im = Image::new(&raw, Point::new((128 - size) / 2, (64 - size) / 2));
        im.draw(display).unwrap();
        display.flush().unwrap();
    }
}
