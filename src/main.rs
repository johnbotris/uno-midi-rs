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
use avr_hal_generic::usart::{Usart, UsartOps};

use millis::*;

const DEFAULT_BAUD_RATE: u32 = 9600;
const MIDI_BAUD_RATE: u32 = 31250;

struct StepParams {
    pub pitch: u8,
    pub velocity: u8,
    pub gate: f32,
}

fn note_on<USART, RX, TX, CLOCK>(
    serial: &mut Usart<USART, RX, TX, CLOCK>,
    channel: u8,
    pitch: u8,
    velocity: u8,
) where
    USART: UsartOps<RX, TX>,
{
    serial.write_byte(0x90 + channel);
    serial.write_byte(pitch);
    serial.write_byte(velocity);
}

fn note_off<USART, RX, TX, CLOCK>(serial: &mut Usart<USART, RX, TX, CLOCK>, channel: u8, pitch: u8)
where
    USART: UsartOps<RX, TX>,
{
    serial.write_byte(0x90 + channel);
    serial.write_byte(pitch);
    serial.write_byte(0);
}

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

    let channel = 0;

    let bpm = 120.0;
    let multiplier = (1.0, 2.0);

    let on = StepParams {
        pitch: 32,
        velocity: 0x45,
        gate: 1.0,
    };

    let off = StepParams {
        pitch: 32,
        velocity: 0,
        gate: 1.0,
    };

    let framerate = 1;

    let mut step_start = millis();
    let mut is_on = true;

    loop {
        let now = millis();

        let beats_per_second = bpm / 60.0;
        let beat_length_ms = 1000.0 / beats_per_second;
        let step_length_ms = beat_length_ms * (multiplier.0 / multiplier.1);

        // ufmt::uwriteln!(&mut serial, "now = {}\r", now).void_unwrap();
        if now - step_start >= step_length_ms as u32 {
            is_on = !is_on;
            step_start = now;

            let (last_step, this_step) = if is_on { (&off, &on) } else { (&on, &off) };

            note_off(&mut serial, channel, last_step.pitch);
            note_on(&mut serial, channel, this_step.pitch, this_step.velocity);
        }
    }
}
