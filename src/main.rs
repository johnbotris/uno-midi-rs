#![no_std]
#![no_main]
// required by millis
#![feature(abi_avr_interrupt)]

mod millis;

extern crate ufmt;

use core::cell::RefCell;

use arduino_uno::adc::{self, Adc};
use arduino_uno::hal::port::mode::Output;
use arduino_uno::hal::port::portb::PB5;
use arduino_uno::prelude::*;
use arduino_uno::{delay_ms, Peripherals, Pins, Serial};
use avr_hal_generic::usart::{Usart, UsartOps};

use millis::*;

const DEFAULT_BAUD_RATE: u32 = 9600;
const MIDI_BAUD_RATE: u32 = 31250;

const ANALOG_IN_MAX: u16 = 1024;

const MAX_STEPS: u16 = 24;

#[arduino_uno::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    millis_init(dp.TC0);

    let mut pins = Pins::new(dp.PORTB, dp.PORTC, dp.PORTD);

    let baud_rate = DEFAULT_BAUD_RATE;

    let mut serial = RefCell::new(Serial::new(
        dp.USART0,
        pins.d0,
        pins.d1.into_output(&mut pins.ddr),
        baud_rate.into_baudrate(),
    ));

    let mut adc: RefCell<Adc> = RefCell::new(Adc::new(dp.ADC, Default::default()));

    macro_rules! analog_read {
        ($pin:expr) => {{
            let value: u16 = nb::block!(adc.borrow_mut().read(&mut $pin)).void_unwrap();
            value
        }};
    }

    let rotation = 0;
    let channel = 0;
    let bpm = 120.0;

    let mut a0 = { pins.a0.into_analog_input(&mut adc.borrow_mut()) };
    let mut get_multiplier = {
        || {
            let multipliers = [(1, 1), (1, 2), (1, 3), (1, 4)];
            let idx = map_analog_value(analog_read!(a0), multipliers.len() as u16);
            multipliers[idx as usize]
        }
    };

    let mut a1 = { pins.a1.into_analog_input(&mut adc.borrow_mut()) };
    let mut get_levels = || {
        let reading = (map_analog_value(analog_read!(a1), 64)) as u8;
        (reading, 64 - reading)
    };

    let mut a2 = { pins.a2.into_analog_input(&mut adc.borrow_mut()) };
    let mut get_num_steps = || map_analog_value(analog_read!(a2), MAX_STEPS);

    let mut a3 = { pins.a3.into_analog_input(&mut adc.borrow_mut()) };
    let mut get_num_onsets = || map_analog_value(analog_read!(a3), MAX_STEPS);

    let mut get_step = |num_steps, num_onsets, current_step| {
        let (on_level, off_level) = get_levels();
        let is_on = euclidean(num_steps, num_onsets, 0, current_step);
        if is_on {
            StepParams {
                pitch: 32,
                velocity: on_level,
                gate: 0.5,
            }
        } else {
            StepParams {
                pitch: 30,
                velocity: off_level,
                gate: 0.5,
            }
        }
    };

    let mut note_on = |pitch, velocity| {
        serial.borrow_mut().write_byte(0x90 + channel);
        serial.borrow_mut().write_byte(pitch);
        serial.borrow_mut().write_byte(velocity);
    };

    let mut note_off = |pitch| {
        serial.borrow_mut().write_byte(0x80 + channel);
        serial.borrow_mut().write_byte(pitch);
        serial.borrow_mut().write_byte(0);
    };

    // Do first step straight away
    let mut previous_step_params = {
        let step = get_step(get_num_steps(), get_num_onsets(), 0);
        note_on(step.pitch, step.velocity);
        step
    };

    // start the timer
    let mut step_start_ms = millis();
    let mut step_counter: u16 = 1;

    loop {
        let now = millis();

        let multiplier = get_multiplier();
        let beats_per_second = bpm / 60.0;
        let beat_length_ms = 1000.0 / beats_per_second;
        let step_length_ms = beat_length_ms as u16 * multiplier.0 / multiplier.1;

        if now - step_start_ms >= step_length_ms as u32 {
            // At the first step, read in all inputs and recalculate parameters
            let num_steps = get_num_steps();
            let num_onsets = get_num_onsets();
            step_start_ms = now;
            step_counter = if num_steps > 0 {
                (step_counter + 1) % num_steps
            } else {
                0
            };

            // TODO send note_off after gate closed rather than on the next step
            note_off(previous_step_params.pitch);

            // Do euclidean rhythm algorithm
            let this_step_params = get_step(num_steps, num_onsets, step_counter);
            note_on(this_step_params.pitch, this_step_params.velocity);

            ufmt::uwriteln!(
                serial.borrow_mut(),
                "num_steps={} num_onsets={} step_counter={} multiplier0={} multiplier1={} step_level={}\r",
                num_steps,
                num_onsets,
                step_counter,
                multiplier.0,
                multiplier.1,
                this_step_params.velocity
            );

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

fn map_analog_value(val: u16, range: u16) -> u16 {
    val * range / ANALOG_IN_MAX
}

struct StepParams {
    pub pitch: u8,
    pub velocity: u8,
    pub gate: f32,
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut serial: arduino_uno::Serial<arduino_uno::hal::port::mode::Floating> =
        unsafe { core::mem::MaybeUninit::uninit().assume_init() };

    ufmt::uwriteln!(&mut serial, "Firmware panic!\r").void_unwrap();

    if let Some(loc) = info.location() {
        ufmt::uwriteln!(
            &mut serial,
            "  At {}:{}:{}\r",
            loc.file(),
            loc.line(),
            loc.column(),
        )
        .void_unwrap();
    }

    loop {
        ufmt::uwriteln!(serial, "im dead\r");
    }
}
