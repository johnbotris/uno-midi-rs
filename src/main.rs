#![no_std]
#![no_main]
// required by millis
#![feature(abi_avr_interrupt)]

mod millis;

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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // TODO Sent note offs or something?
    loop {}
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
    let multiplier = (1.0, 4.0);

    let on = StepParams {
        pitch: 0x24,
        velocity: 0x7f,
        gate: 1.0,
    };

    let off = StepParams {
        pitch: 0x20,
        velocity: 0x0f,
        // TODO Send note off actual gate close
        gate: 1.0,
    };

    let framerate = 1;

    let mut step_start = millis();
    let mut is_on = true;
    let mut current_step: u16 = 0;
    let mut last_step = None;

    loop {
        let now = millis();

        let beats_per_second = bpm / 60.0;
        let beat_length_ms = 1000.0 / beats_per_second;
        let step_length_ms = beat_length_ms * (multiplier.0 / multiplier.1);

        if now - step_start >= step_length_ms as u32 {
            step_start = now;

            let steps = 10;
            let onsets = 3;
            let rotation = 0;

            // Do euclidean rhythm algorithm
            is_on = {
                if steps == 0 || onsets == 0 {
                    false
                } else if onsets >= steps {
                    true
                } else {
                    // Modulo of the target step
                    // Note to self, we can't use u32/i32 because it breaks modulo:
                    // https://github.com/rust-lang/rust/issues/82242
                    // see: further down
                    let target_step = {
                        let c = current_step as i16 - (rotation + 1);
                        let m = steps as i16;
                        let modulo = ((c % m) + m) % m;
                        modulo
                    };
                    let mut bucket = 0;
                    let mut is_on = false;
                    for _ in 0..=target_step {
                        is_on = false;
                        bucket += onsets;
                        if bucket >= steps {
                            bucket -= steps;
                            is_on = true
                        }
                    }
                    is_on
                }
            };

            current_step = (current_step + 1) % steps;

            if let Some(&StepParams { pitch, .. }) = last_step {
                note_off(&mut serial, channel, pitch);
            }

            let this_step = if is_on { &on } else { &off };
            note_on(&mut serial, channel, this_step.pitch, this_step.velocity);

            last_step = Some(this_step);
        }
    }
}
