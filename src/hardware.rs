use arduino_uno::hal::port::portc::{PC0, PC1, PC2, PC3};
use arduino_uno::prelude::*;
use arduino_uno::{adc::Adc, pac::TC0, Serial};
use arduino_uno::{Peripherals, Pins};
use avr_hal_generic::{
    clock::MHz16,
    port::mode::{Analog, Floating},
    usart::Baudrate,
};
use uno_mux::{Multiplexer, U4};

type SerialType = Serial<Floating>;

type Mux = Multiplexer<
    PD0<Input<Floating>>,
    PD1<Input<Floating>>,
    PD2<Input<Floating>>,
    PD3<Input<Floating>>,
    PC0<Input<Analog>>,
    (),
>;

pub struct Hardware {
    adc: Adc,
    serial: SerialType,
    mux: Mux,
    tc0: TC0,
}

impl Hardware {
    pub fn new(serial_baudrate: Baudrate<MHz16>) -> Self {
        let dp = Peripherals::take().unwrap();
        let mut pins = Pins::new(dp.PORTB, dp.PORTC, dp.PORTD);
        let mut adc = Adc::new(dp.ADC, Default::default());

        let mux = Mux::new(
            pins.d0.into_output(&mut pins.ddr),
            pins.d1.into_output(&mut pins.ddr),
            pins.d2.into_output(&mut pins.ddr),
            pins.d3.into_output(&mut pins.ddr),
            pins.a0.into_analog_input(&mut adc),
            (),
        );

        let serial = Serial::new(
            dp.USART0,
            pins.d0,
            pins.d1.into_output(&mut pins.ddr),
            serial_baudrate,
        );

        Self {
            adc,
            serial,
            mux,
            tc0: dp.TC0,
        }
    }

    pub fn tc0(&mut self) -> &mut TC0 {
        &mut self.tc0
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.serial.write_byte(byte)
    }

    pub fn mux_read(&mut self, channel: U4) -> &mut Mux {
        nb::block!(self.adc.read(self.mux.pin(channel)))
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
