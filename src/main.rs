#![no_std]
#![no_main]
// required by millis
#![feature(abi_avr_interrupt)]

mod millis;

extern crate ufmt;

use arduino_uno::adc;
use arduino_uno::hal::port::mode::Output;
use arduino_uno::hal::port::portb::PB5;
use arduino_uno::prelude::*;
use arduino_uno::{delay_ms, Peripherals, Pins, Serial};
use avr_hal_generic::usart::{Usart, UsartOps};

use millis::*;

const DEFAULT_BAUD_RATE: u32 = 9600;
const MIDI_BAUD_RATE: u32 = 31250;
const ANALOG_IN_MAX: u16 = 1024;

const MULTIPLIERS: [(u16, u16); 4] = [(1, 1), (1, 2), (1, 3), (1, 4)];

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
    let mut adc = adc::Adc::new(dp.ADC, Default::default());
    let mut rate_pot = pins.a0.into_analog_input(&mut adc);

    macro_rules! analog_read {
        ($pin:expr) => {{
            let value: u16 = nb::block!(adc.read(&mut $pin)).void_unwrap();
            value
        }};
    }

    // TODO modify these with analog or digital inputs
    let num_steps: u16 = 8;
    let onsets = 3;
    let rotation = 0;
    let channel = 0;
    let bpm = 120.0;

    macro_rules! get_multiplier {
        () => {{
            let idx = analog_read!(rate_pot) * MULTIPLIERS.len() as u16 / ANALOG_IN_MAX;
            MULTIPLIERS[idx as usize]
        }};
    }

    let mut multiplier = get_multiplier!();

    macro_rules! note_on {
        ($pitch:expr, $velocity:expr) => {
            serial.write_byte(0x90 + channel);
            serial.write_byte($pitch);
            serial.write_byte($velocity);
        };
    }

    macro_rules! note_off {
        ($pitch:expr) => {
            serial.write_byte(0x80 + channel);
            serial.write_byte($pitch);
            serial.write_byte(0);
        };
    }

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

    // Do first step straight away
    let mut previous_step_params = {
        let is_on = euclidean(num_steps, onsets, rotation, 0);
        let step = if is_on { &on } else { &off };
        note_on!(step.pitch, step.velocity);
        step
    };

    // start the timer
    let mut step_start_ms = millis();
    let mut step_counter: u16 = 1;

    loop {
        let now = millis();

        let beats_per_second = bpm / 60.0;
        let beat_length_ms = 1000.0 / beats_per_second;
        let step_length_ms = beat_length_ms as u16 * multiplier.0 / multiplier.1;

        if now - step_start_ms >= step_length_ms as u32 {
            // At the first step, read in all inputs and recalculate parameters
            multiplier = get_multiplier!();
            step_start_ms = now;
            step_counter = (step_counter + 1) % num_steps;

            // TODO send note_off after gate closed rather than on the next step
            note_off!(previous_step_params.pitch);

            // Do euclidean rhythm algorithm
            let this_step_params = if euclidean(num_steps, onsets, rotation, step_counter) {
                &on
            } else {
                &off
            };

            note_on!(this_step_params.pitch, this_step_params.velocity);

            previous_step_params = this_step_params;
        }
    }
}

fn euclidean(steps: u16, onsets: u16, rotation: i16, current_step: u16) -> bool {
    if steps == 0 || onsets == 0 {
        false
    } else if onsets >= steps {
        true
    } else {
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
}

struct StepParams {
    pub pitch: u8,
    pub velocity: u8,
    pub gate: f32,
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // TODO Sent note offs or something?
    loop {}
}
