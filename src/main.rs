#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)] // required by millis

mod hardware;
mod millis;

extern crate ufmt;

use core::{
    cell::RefCell,
    ops::{Div, Mul},
};

use arduino_uno::prelude::*;
use hardware::*;
use millis::*;

use uno_mux::U4;

const DEFAULT_BAUD_RATE: u32 = 9600;
const MIDI_BAUD_RATE: u32 = 31250;
const ANALOG_IN_MAX: u16 = 1024;
const MAX_STEPS: u16 = 16;

#[arduino_uno::entry]
fn main() -> ! {
    let hardware: RefCell<Hardware> = RefCell::new(Hardware::new(MIDI_BAUD_RATE.into_baudrate()));

    millis_init(hardware.borrow_mut().tc0());

    let rotation = 0;
    let channel = 0;
    let bpm = 120.0;

    let get_multiplier = {
        || {
            let multipliers = [(1, 1), (1, 2), (1, 3), (1, 4)];
            let idx = map_analog_value(
                hardware.borrow_mut().mux_read(U4::ZERO),
                multipliers.len() as u16,
            );
            multipliers[idx as usize]
        }
    };

    let get_levels = || {
        let reading = map_range(
            hardware.borrow_mut().mux_read(U4::ONE) as u32,
            ANALOG_IN_MAX,
            255,
        );

        let on = clamp(255 - reading, 0, 127);
        let off = clamp(reading, 0, 127);

        (on, off)
    };

    let get_num_steps = || map_analog_value(hardware.borrow_mut().mux_read(U4::TWO), MAX_STEPS);

    let get_num_onsets = || map_analog_value(hardware.borrow_mut().mux_read(U4::THREE), MAX_STEPS);

    let get_step = |num_steps, num_onsets, current_step| {
        let (on_level, off_level) = get_levels();
        let is_on = euclidean(num_steps, num_onsets, 0, current_step);
        if is_on {
            StepParams {
                pitch: 30,
                velocity: on_level,
                gate: 0.5,
            }
        } else {
            StepParams {
                pitch: 42,
                velocity: off_level,
                gate: 0.5,
            }
        }
    };

    let note_on = |pitch, velocity| {
        let mut hardware = hardware.borrow_mut();
        hardware.write_byte(0x90 + channel);
        hardware.write_byte(pitch);
        hardware.write_byte(velocity);
    };

    let mut note_off = |pitch| {
        let mut hardware = hardware.borrow_mut();
        hardware.write_byte(0x80 + channel);
        hardware.write_byte(pitch);
        hardware.write_byte(0);
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

            // ufmt::uwrite!(hardware.borrow_mut(), "\r                                                                                                                       \r");
            // ufmt::uwrite!(
            //     hardware.borrow_mut(),
            //     "num_steps={} num_onsets={} step_counter={} multiplier0={} multiplier1={} step_level={}",
            //     num_steps,
            //     num_onsets,
            //     step_counter,
            //     multiplier.0,
            //     multiplier.1,
            //     this_step_params.velocity
            // );

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

fn clamp<T>(val: T, low: T, hi: T) -> T {
    if val < lo {
        low
    } else if val > hi {
        high
    } else {
        val
    }
}

fn map_range<T>(val: T, from: T, to: T) -> T
where
    T: Mul + Mul<Output = T> + Div + Div<Output = T>,
{
    val * to / from
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
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // let mut serial: arduino_uno::Serial<arduino_uno::hal::port::mode::Floating> =
    //     unsafe { core::mem::MaybeUninit::uninit().assume_init() };

    // ufmt::uwriteln!(&mut serial, "Firmware panic!\r").void_unwrap();

    // if let Some(loc) = info.location() {
    //     ufmt::uwriteln!(
    //         &mut serial,
    //         "  At {}:{}:{}\r",
    //         loc.file(),
    //         loc.line(),
    //         loc.column(),
    //     )
    //     .void_unwrap();
    // }

    loop {}
}
