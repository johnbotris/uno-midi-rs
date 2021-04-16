#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)] // required by millis

mod hardware;
mod millis;

extern crate ufmt;

use core::ops::{Div, Mul};

use arduino_uno::prelude::*;
use hardware::*;
use millis::*;

use uno_mux::u4::U4;

const DEFAULT_BAUD_RATE: u32 = 9600;
const MIDI_BAUD_RATE: u32 = 31250;
const ANALOG_IN_MAX: u16 = 1024;
const MAX_STEPS: u16 = 16;

const MULTIPLIER_CHANNEL: U4 = U4::ZERO;
const LEVELS_CHANNEL: U4 = U4::ONE;
const STEPS_CHANNEL: U4 = U4::TWO;
const ONSETS_CHANNEL: U4 = U4::THREE;
const TEMPO_CHANNEL: U4 = U4::FOUR;
const ON_PITCH_CHANNEL: U4 = U4::FIVE;
const OFF_PITCH_CHANNEL: U4 = U4::SIX;
const GATE_CHANNEL: U4 = U4::SEVEN;

#[arduino_uno::entry]
fn main() -> ! {
    let mut hardware: Hardware = Hardware::new(MIDI_BAUD_RATE.into_baudrate());

    millis_init(hardware.tc0());

    let rotation = 0;
    let channel = 0;
    let bpm = 120.0;

    // Do first step straight away
    let mut previous_step_params = {
        let num_steps = get_num_steps(&mut hardware);
        let num_onsets = get_num_onsets(&mut hardware);
        let step = get_step(&mut hardware, num_steps, num_onsets, 0);
        note_on(&mut hardware, channel, step.pitch, step.velocity);
        step
    };

    // start the timer
    let mut step_start_ms = millis();
    let mut step_counter: u16 = 1;

    loop {
        let now = millis();

        let multiplier = get_multiplier(&mut hardware);
        let beats_per_second = bpm / 60.0;
        let beat_length_ms = 1000.0 / beats_per_second;
        let step_length_ms = beat_length_ms as u16 * multiplier.0 / multiplier.1;

        if now - step_start_ms >= step_length_ms as u32 {
            // At the first step, read in all inputs and recalculate parameters
            let num_steps = get_num_steps(&mut hardware);
            let num_onsets = get_num_onsets(&mut hardware);
            step_start_ms = now;
            step_counter = if num_steps > 0 {
                (step_counter + 1) % num_steps
            } else {
                0
            };

            // TODO send note_off after gate closed rather than on the next step
            note_off(&mut hardware, channel, previous_step_params.pitch);

            // Do euclidean rhythm algorithm
            let this_step_params = get_step(&mut hardware, num_steps, num_onsets, step_counter);
            note_on(
                &mut hardware,
                channel,
                this_step_params.pitch,
                this_step_params.velocity,
            );

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

fn get_multiplier(hardware: &mut Hardware) -> (u16, u16) {
    let multipliers = [(1, 1), (1, 2), (1, 3), (1, 4)];
    let idx = map_analog_value(
        hardware.mux_read(MULTIPLIER_CHANNEL),
        multipliers.len() as u16,
    );
    multipliers[idx as usize]
}

fn get_levels(hardware: &mut Hardware) -> (u8, u8) {
    let reading = map_range(
        hardware.mux_read(LEVELS_CHANNEL) as u32,
        ANALOG_IN_MAX.into(),
        255,
    );
    let on = clamp(255 - reading, 0, 127);
    let off = clamp(reading, 0, 127);
    (on as u8, off as u8)
}

fn get_num_steps(hardware: &mut Hardware) -> u16 {
    map_analog_value(hardware.mux_read(STEPS_CHANNEL), MAX_STEPS)
}

fn get_num_onsets(hardware: &mut Hardware) -> u16 {
    map_analog_value(hardware.mux_read(ONSETS_CHANNEL), MAX_STEPS)
}

fn get_step(
    hardware: &mut Hardware,
    num_steps: u16,
    num_onsets: u16,
    current_step: u16,
) -> StepParams {
    let (on_level, off_level) = get_levels(hardware);
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
}
fn note_on(hardware: &mut Hardware, channel: u8, pitch: u8, velocity: u8) {
    hardware.write_byte(0x90 + channel);
    hardware.write_byte(pitch);
    hardware.write_byte(velocity);
}

fn note_off(hardware: &mut Hardware, channel: u8, pitch: u8) {
    hardware.write_byte(0x80 + channel);
    hardware.write_byte(pitch);
    hardware.write_byte(0);
}

fn clamp<T>(val: T, low: T, high: T) -> T
where
    T: PartialOrd,
{
    if val <= low {
        low
    } else if val >= high {
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
