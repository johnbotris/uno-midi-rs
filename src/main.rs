#![no_std]
#![no_main]
// required by millis
#![feature(abi_avr_interrupt)]

mod maths;
mod millis;
mod patterns;

extern crate panic_halt;
extern crate ufmt;

use arduino_uno::hal::port::mode::Output;
use arduino_uno::hal::port::portb::PB5;
use arduino_uno::prelude::*;
use arduino_uno::{delay_ms, Peripherals, Pins, Serial};

use millis::*;

const DEFAULT_BAUD_RATE: u32 = 9600;
const MIDI_BAUD_RATE: u32 = 31250;

#[arduino_uno::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    millis_init(dp.TC0);

    let mut pins = Pins::new(dp.PORTB, dp.PORTC, dp.PORTD);

    let mut serial = Serial::new(
        dp.USART0,
        pins.d0,
        pins.d1.into_output(&mut pins.ddr),
        MIDI_BAUD_RATE.into_baudrate(),
    );

    let mut note_on = |pitch, velocity| {
        serial.write_byte(0x90);
        serial.write_byte(pitch);
        serial.write_byte(velocity);
    };

    let bpm = 120.0;

    let framerate = 1;
    let ms_per_frame = 1000 / framerate;
    let mut frame_start = millis();

    let mut is_on = true;

    loop {
        let now = millis();

        // ufmt::uwriteln!(&mut serial, "now = {}\r", now).void_unwrap();
        if now - frame_start >= ms_per_frame {
            let velocity = is_on as u8 * 0x45;
            is_on = !is_on;
            frame_start = now;
            note_on(32, velocity);
        }
    }
}
