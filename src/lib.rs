//! Loosely adapted from https://github.com/david-sawatzke/hub75-rs

#![no_std]

mod fmt;

pub mod pins;
use pins::*;

#[cfg(feature = "hal-02")]
use embedded_hal_02::blocking::delay::DelayUs;
#[cfg(feature = "hal-1")]
use embedded_hal_1::delay::DelayNs;

#[cfg(feature = "hal-02")]
pub trait DelayProvider: DelayUs<u32> {}
#[cfg(feature = "hal-1")]
pub trait DelayProvider: DelayNs {}

#[cfg(feature = "hal-02")]
impl<T: DelayUs<u32>> DelayProvider for T {}
#[cfg(feature = "hal-1")]
impl<T: DelayNs> DelayProvider for T {}

/// A helper struct for computing the frame time compensation to maintain a constant
/// brightness across all color depths.
struct FrameTimeCompensation<const BITS: u8> {
    h: u32,
}

impl<const BITS: u8> FrameTimeCompensation<BITS> {
    fn new(on_ratio: f64) -> Self {
        assert!((0f64..1f64).contains(&on_ratio));

        let p = (2 * BITS + 1) as f64;
        let h = ((p * on_ratio) / (1. - on_ratio)) as u32;

        fmt::trace!("FTC H constant: {}", h);

        Self { h }
    }

    const fn duration(&self, mask: &u8) -> u32 {
        2u32.pow(*mask as u32) * self.h / (2u32.pow(BITS as u32) - 1)
    }
}

// Display Drivers

/// A 64x32 display with 2 colors written at a time.
pub struct Hub75_64_32_2<
    const BITS: u8,
    UpperColorPins: IsColorPins,
    LowerColorPins: IsColorPins,
    RowPins: IsRowPins,
    DataPins: IsDataPins,
> {
    top_data: [[(u8, u8, u8); 64]; 32 / 2],
    bottom_data: [[(u8, u8, u8); 64]; 32 / 2],
    ftc: FrameTimeCompensation<BITS>,
    upper_color_pins: UpperColorPins,
    lower_color_pins: LowerColorPins,
    row_pins: RowPins,
    data_pins: DataPins,
}

impl<E, const BITS: u8, UpperColorPins, LowerColorPins, RowPins, DataPins>
    Hub75_64_32_2<BITS, UpperColorPins, LowerColorPins, RowPins, DataPins>
where
    UpperColorPins: IsColorPins<Error = E>,
    LowerColorPins: IsColorPins<Error = E>,
    RowPins: IsRowPins<Error = E>,
    DataPins: IsDataPins<Error = E>,
{
    /// Construct a new Hub75x display instance.
    ///
    /// `on_ratio` is a float from 0-1 (exclusive) that configures the proportion
    /// with which the pixel values are held before proceeding to the next row.
    /// This permits control of the observed brightness of the display at the cost
    /// of refresh rate.
    pub fn new(
        upper_color_pins: UpperColorPins,
        lower_color_pins: LowerColorPins,
        row_pins: RowPins,
        data_pins: DataPins,
        on_ratio: f64,
    ) -> Self {
        let ftc = FrameTimeCompensation::new(on_ratio);

        fmt::trace!("new Hub75_64_32_2 with {} bits", BITS);

        Self {
            top_data: [[(0, 0, 0); 64]; 16],
            bottom_data: [[(0, 0, 0); 64]; 16],
            ftc,
            upper_color_pins,
            lower_color_pins,
            row_pins,
            data_pins,
        }
    }

    /// Output the framebuffer to the display.
    ///
    /// *This function is time-sensitive and should be called as often as possible.*
    pub fn output<Delay: DelayProvider>(&mut self, delay: &mut Delay) -> Result<(), E> {
        for (i, (upper_row, lower_row)) in self.top_data.iter().zip(&self.bottom_data).enumerate() {
            self.row_pins.set_row(&(i as u8))?;

            for mask in 0..BITS {
                for (upper_col, lower_col) in upper_row.iter().zip(lower_row) {
                    self.upper_color_pins.set_color::<BITS>(upper_col, &mask)?;
                    self.lower_color_pins.set_color::<BITS>(lower_col, &mask)?;

                    self.data_pins.shift(delay)?;
                }

                self.data_pins.latch(delay)?;

                self.data_pins.show(delay, self.ftc.duration(&mask))?;
            }
        }

        Ok(())
    }

    /// Set the framebuffer to all black.
    pub fn wipe(&mut self) {
        self.top_data = [[(0, 0, 0); 64]; 16];
        self.bottom_data = [[(0, 0, 0); 64]; 16];
    }
}

// DrawTarget impl

use core::convert::Infallible;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    pixelcolor::{Rgb565, RgbColor},
    primitives::Rectangle,
    Pixel,
};

impl<
        const BITS: u8,
        UpperColorPins: IsColorPins,
        LowerColorPins: IsColorPins,
        RowPins: IsRowPins,
        DataPins: IsDataPins,
    > Dimensions for Hub75_64_32_2<BITS, UpperColorPins, LowerColorPins, RowPins, DataPins>
{
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(Point::zero(), Size::new(64, 32))
    }
}

impl<
        const BITS: u8,
        UpperColorPins: IsColorPins,
        LowerColorPins: IsColorPins,
        RowPins: IsRowPins,
        DataPins: IsDataPins,
    > DrawTarget for Hub75_64_32_2<BITS, UpperColorPins, LowerColorPins, RowPins, DataPins>
{
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        // This table remaps linear input values
        // (the numbers we’d like to use; e.g. 127 = half brightness)
        // to nonlinear gamma-corrected output values
        // (numbers producing the desired effect on the LED;
        // e.g. 36 = half brightness).
        const GAMMA8: [u8; 256] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4,
            4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11,
            12, 12, 13, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22,
            22, 23, 24, 24, 25, 25, 26, 27, 27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37,
            38, 39, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58,
            59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72, 73, 74, 75, 77, 78, 79, 81, 82, 83, 85,
            86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104, 105, 107, 109, 110, 112, 114,
            115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137, 138, 140, 142, 144,
            146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175, 177, 180,
            182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
            223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
        ];

        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.x < 64 && coord.y >= 0 && coord.y < 32 {
                if coord.y < 16 {
                    self.top_data[coord.y as usize][coord.x as usize] = (
                        GAMMA8[(color.r() as usize + 1) * 8 - 1],
                        GAMMA8[(color.g() as usize + 1) * 4 - 1],
                        GAMMA8[(color.b() as usize + 1) * 8 - 1],
                    );
                } else {
                    self.bottom_data[(coord.y - 16) as usize][coord.x as usize] = (
                        GAMMA8[(color.r() as usize + 1) * 8 - 1],
                        GAMMA8[(color.g() as usize + 1) * 4 - 1],
                        GAMMA8[(color.b() as usize + 1) * 8 - 1],
                    );
                }
            }
        }

        Ok(())
    }
}
