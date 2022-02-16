//! # LMX2594 initialization
//!
//! This program initializes an LMX2594 evaluation board using a
//! Raspberry Pi Pico.
//!
//! ```text
//! Raspberry Pi Pico Pinout
//! ========================
//!
//! | Pin | Purpose     |
//! +-----+-------------+
//! |  4  | SPI0 SCK    |
//! |  5  | SPI0 TX     |
//! |  6  | SPI0 RX     |
//! |  7  | SPI0 CSn    |
//! |  8  | GND         |
//! |  9  | Chip Enable |
//!
//! LMX2594EVM uWire Pins
//! =====================
//!
//! | No. | Name       | SPI Pin | Misc. Pin   |
//! |-----+------------+---------+-------------|
//! |   1 | RAMPDIR/CE |         | Chip enable |
//! |   2 | CSB        | ~SS     |             |
//! |   3 | MUXout     | MISO    |             |
//! |   4 | SDI        | MOSI    |             |
//! |   5 | NC         |         |             |
//! |   6 | GND        |         | GND         |
//! |   7 | RampCLK    |         |             |
//! |   8 | SCK        | SCLK    |             |
//! |   9 | SysRefReq  |         |             |
//! |  10 | SYNC       |         |             |
//!
//! LMX2594 uWire Pinout
//! ====================
//!
//! +---+---+---+---+---+
//! | 2 | 4 | 6 | 8 | 10|
//! +---+---+---+---+---+
//! | 1 | 3 | 5 | 7 | 9 |
//! +---+===+===+===+---+
//!         Notch
//! #+end_example

//! ```

#![no_std]
#![no_main]

// Following rp-hal/boards/rp-pico/examples/pico_spi_sd_card.rs
// https://github.com/rp-rs/rp-hal/blob/main/boards/rp-pico/examples/pico_spi_sd_card.rs

// The macro for our start-up function
use cortex_m_rt::entry;

// info!() and error!() macros for printing information to the debug output
use defmt::*;
use defmt_rtt as _;

use embedded_hal::digital::v2::OutputPin;
// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// Pull in any important traits
use rp_pico::hal::prelude::*;

// Embed the `Hz` function/trait:
use embedded_time::rate::*;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use rp_pico::hal::pac;

// Import the SPI abstraction:
use rp_pico::hal::spi;

// Import the GPIO abstraction:
use rp_pico::hal::gpio;

// A shorter alias for the Hardware Abstraction Layer, which provides
// higher-level drivers.
use rp_pico::hal;

mod lmx2594;

use lmx2594::{Lmx2594, FCAL_EN_OFF, FCAL_EN_ON, REG_MAP, RESET_OFF, RESET_ON};

#[entry]
fn main() -> ! {
    info!("Program start");

    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Set the LED to be an output
    let mut led_pin = pins.led.into_push_pull_output();

    // These are implicitly used by the spi driver if they are in the correct mode
    let _spi_sclk = pins.gpio2.into_mode::<gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio3.into_mode::<gpio::FunctionSpi>();
    let _spi_miso = pins.gpio4.into_mode::<gpio::FunctionSpi>();
    let mut spi_cs = pins.gpio5.into_push_pull_output();

    // This pin will be used for Chip Enable on the LMX 2594
    // (overall power-on, not SPI chip select)
    let mut ce_pin = pins.gpio6.into_push_pull_output();

    // Create an SPI driver instance for the SPI0 device
    let spi = spi::Spi::<_, _, 8>::new(pac.SPI0);

    // Exchange the uninitialised SPI driver for an initialised one
    let mut spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        1_000_000u32.Hz(),
        &embedded_hal::spi::MODE_0,
    );

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    // Initialize the LMX2594

    // Turn on the LED while we initialize
    led_pin.set_high().unwrap();

    // Ensure the ~CS pin is high before power-on
    spi_cs.set_high().unwrap();
    delay.delay_ms(10);

    // Power on the device
    ce_pin.set_high().unwrap();
    delay.delay_ms(10);

    let mut buf: [u8; 3] = [0; 3];

    RESET_ON.write_reg(&mut spi, &mut spi_cs, &mut buf);
    delay.delay_ms(10);

    RESET_OFF.write_reg(&mut spi, &mut spi_cs, &mut buf);
    delay.delay_ms(10);

    for r in REG_MAP.iter().rev() {
        r.write_reg(&mut spi, &mut spi_cs, &mut buf);
        delay.delay_ms(10);
    }
    delay.delay_ms(10);

    FCAL_EN_ON.write_reg(&mut spi, &mut spi_cs, &mut buf);
    delay.delay_ms(10);

    FCAL_EN_OFF.write_reg(&mut spi, &mut spi_cs, &mut buf);
    delay.delay_ms(10);

    led_pin.set_low().unwrap();

    #[allow(clippy::empty_loop)]
    loop {
        //FCAL_EN_ON.write_reg(&mut spi, &mut spi_cs, &mut buf);
        //delay.delay_us(100);
    }
}
