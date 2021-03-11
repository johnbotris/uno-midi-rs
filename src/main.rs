#![no_std]
#![no_main]

extern crate panic_halt;
use arduino_uno::hal::port::mode::Output;
use arduino_uno::hal::port::portb::PB5;
use arduino_uno::prelude::*;
use arduino_uno::{delay_ms, Peripherals, Pins, Serial};

#[arduino_uno::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let mut pins = Pins::new(dp.PORTB, dp.PORTC, dp.PORTD);

    // let mut midi = pins.d1.into_output(&mut pins.ddr);

    let mut serial = Serial::new(
        dp.USART0,
        pins.d0,
        pins.d1.into_output(&mut pins.ddr),
        31250.into_baudrate(),
    );

    let mut note_on = |pitch, velocity| {
        serial.write_byte(0x90);
        serial.write_byte(pitch);
        serial.write_byte(velocity);
    };

    loop {
        note_on(0x1e, 0x45);
        delay_ms(100u16);

        note_on(0x1e, 0x00);
        delay_ms(100u16);
    }
}
