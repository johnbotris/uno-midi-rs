use arduino_uno::hal::port::portc::{PC0, PC1, PC2, PC3};
use arduino_uno::prelude::*;
use arduino_uno::{adc::Adc, pac::TC0, Serial};
use arduino_uno::{Peripherals, Pins};
use avr_hal_generic::{
    clock::MHz16,
    port::mode::{Analog, Floating},
    usart::Baudrate,
};
use ufmt::{uDisplay, Formatter};

type SerialType = Serial<Floating>;

pub struct Hardware {
    adc: Adc,
    serial: SerialType,

    a0: PC0<Analog>,
    a1: PC1<Analog>,
    a2: PC2<Analog>,
    a3: PC3<Analog>,
    tc0: TC0,
}

impl Hardware {
    pub fn new(serial_baudrate: Baudrate<MHz16>) -> Self {
        let dp = Peripherals::take().unwrap();
        let mut pins = Pins::new(dp.PORTB, dp.PORTC, dp.PORTD);
        let mut adc = Adc::new(dp.ADC, Default::default());

        let a0 = pins.a0.into_analog_input(&mut adc);
        let a1 = pins.a1.into_analog_input(&mut adc);
        let a2 = pins.a2.into_analog_input(&mut adc);
        let a3 = pins.a3.into_analog_input(&mut adc);

        let serial = Serial::new(
            dp.USART0,
            pins.d0,
            pins.d1.into_output(&mut pins.ddr),
            serial_baudrate,
        );

        Self {
            adc,
            serial,
            a0,
            a1,
            a2,
            a3,
            tc0: dp.TC0,
        }
    }

    pub fn read_a0(&mut self) -> u16 {
        nb::block!(self.adc.read(&mut self.a0)).void_unwrap()
    }

    pub fn read_a1(&mut self) -> u16 {
        nb::block!(self.adc.read(&mut self.a1)).void_unwrap()
    }

    pub fn read_a2(&mut self) -> u16 {
        nb::block!(self.adc.read(&mut self.a2)).void_unwrap()
    }

    pub fn read_a3(&mut self) -> u16 {
        nb::block!(self.adc.read(&mut self.a3)).void_unwrap()
    }

    pub fn tc0(&mut self) -> &mut TC0 {
        &mut self.tc0
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.serial.write_byte(byte)
    }
}

impl ufmt::uWrite for Hardware {
    type Error = <SerialType as ufmt::uWrite>::Error;
    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        self.serial.write_str(s)
    }

    fn write_char(&mut self, c: char) -> Result<(), Self::Error> {
        self.serial.write_char(c)
    }
}
