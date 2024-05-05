#[cfg(feature = "hal-02")]
use embedded_hal_02::digital::v2::{OutputPin, PinState};
#[cfg(feature = "hal-1")]
use embedded_hal_1::digital::{OutputPin, PinState};

use crate::DelayProvider;

// Traits

/// Represents pins that control the set color to be written to the corresponding
/// row of the display.
pub trait IsColorPins<Color = (u8, u8, u8), Mask = u8> {
    type Error;

    /// Set the pin states to represent the provided color, color depth, and color
    /// mask.
    fn set_color<const BITS: u8>(&mut self, color: &Color, mask: &Mask) -> Result<(), Self::Error>;
}

/// Represents pins that control the row of the display to be written to.
pub trait IsRowPins<Row = u8> {
    type Error;

    /// Set the pin states to represent the provided row selection.
    fn set_row(&mut self, row: &Row) -> Result<(), Self::Error>;
}

/// Represents pins that control the transfer of data to the display.
pub trait IsDataPins {
    type Error;

    /// Toggle the clock pin appropriately to shift one "datum".
    fn shift<Delay: DelayProvider>(&mut self, delay: &mut Delay) -> Result<(), Self::Error>;

    /// Toggle the latch pin to confirm the shifted values.
    fn latch<Delay: DelayProvider>(&mut self, delay: &mut Delay) -> Result<(), Self::Error>;

    /// Toggle the output enable pin to display the registered pixel values of the selected
    /// row for the provided duration.
    fn show<Delay: DelayProvider>(
        &mut self,
        delay: &mut Delay,
        duration: u32, /* defined by DelayNs */
    ) -> Result<(), Self::Error>;
}

// Impls
// TODO: macro generation?

/// Standard three R, G, B color pins.
impl<E, R, G, B> IsColorPins for (R, G, B)
where
    R: OutputPin<Error = E>,
    G: OutputPin<Error = E>,
    B: OutputPin<Error = E>,
{
    type Error = E;

    fn set_color<const BITS: u8>(
        &mut self,
        color: &(u8, u8, u8),
        mask: &u8,
    ) -> Result<(), Self::Error> {
        self.0
            .set_state(if (color.0 >> (mask + 8 - BITS)) & 0x1 == 1 {
                PinState::High
            } else {
                PinState::Low
            })?;
        self.1
            .set_state(if (color.1 >> (mask + 8 - BITS)) & 0x1 == 1 {
                PinState::High
            } else {
                PinState::Low
            })?;
        self.2
            .set_state(if (color.2 >> (mask + 8 - BITS)) & 0x1 == 1 {
                PinState::High
            } else {
                PinState::Low
            })?;

        Ok(())
    }
}

/// 4 Row control pins for 16 (2^4) rows.
impl<E, A, B, C, D> IsRowPins for (A, B, C, D)
where
    A: OutputPin<Error = E>,
    B: OutputPin<Error = E>,
    C: OutputPin<Error = E>,
    D: OutputPin<Error = E>,
{
    type Error = E;

    fn set_row(&mut self, row: &u8) -> Result<(), Self::Error> {
        self.0.set_state(if row & 0x1 == 0 {
            PinState::Low
        } else {
            PinState::High
        })?;

        self.1.set_state(if (row >> 1) & 0x1 == 0 {
            PinState::Low
        } else {
            PinState::High
        })?;

        self.2.set_state(if (row >> 2) & 0x1 == 0 {
            PinState::Low
        } else {
            PinState::High
        })?;

        self.3.set_state(if (row >> 3) & 0x1 == 0 {
            PinState::Low
        } else {
            PinState::High
        })?;

        Ok(())
    }
}

/// Standard data pins: clock, latch, and output enable.
impl<E, Clk, Latch, Output> IsDataPins for (Clk, Latch, Output)
where
    Clk: OutputPin<Error = E>,
    Latch: OutputPin<Error = E>,
    Output: OutputPin<Error = E>,
{
    type Error = E;

    fn shift<Delay: DelayProvider>(&mut self, delay: &mut Delay) -> Result<(), E> {
        self.0.set_high()?;
        delay.delay_us(1);
        self.0.set_low()?;
        delay.delay_us(1);

        Ok(())
    }

    fn latch<Delay: DelayProvider>(&mut self, delay: &mut Delay) -> Result<(), E> {
        self.1.set_high()?;
        delay.delay_us(1);
        self.1.set_low()?;

        Ok(())
    }

    fn show<Delay: DelayProvider>(
        &mut self,
        delay: &mut Delay,
        duration: u32, /* defined by DelayNs */
    ) -> Result<(), E> {
        self.2.set_low()?;
        delay.delay_us(duration);
        self.2.set_high()?;

        Ok(())
    }
}
