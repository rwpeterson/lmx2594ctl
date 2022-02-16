//! LMX2594 programmed using 24-bit shift registers:
//! MSB <- [R/W bit, 0 writes] [7-bit address] [16-bit data field] -> LSB
//! Recommended power-up sequence:
//! 1. Apply power to device
//! 2. Program RESET = 1 to reset regs
//! 3. Program RESET = 0 to remove reset
//! 4. Program regs as shown in register map, highest to lowest
//! 5. Wait 10 ms
//! 6. Program R0 one more time with FCAL_EN = 1 to ensure VCO cal
//!    is run from a stable state
//! Recommended changing frequency sequence:
//! 1. Change N-divider value
//! 2. Change PLL numerator and denominator
//! 3. Program FCAL_EN = 1
//! General programming remarks
//! 1. Registers without field names in register map must be programmed as shown
//! 2. Not all registers need to be programmed:
//!    * R107-R112 are readback only, do not need to be programmed
//!    * R79-R106 need to be programmed only if ramping function RAMP_EN is used
//!    * R0-R78 must always be programmed (lines 35-113 in TICS Pro hex dump)

use embedded_hal::{digital::v2::OutputPin, prelude::_embedded_hal_blocking_spi_Write};
use rp_pico::hal::{
    gpio::{bank0::Gpio5, Output, Pin, PushPull},
    pac::SPI0,
    spi::{Enabled, Spi},
};

pub static REG_MAP: [u32; 113] = [
    0x00241c, // 0
    0x010808, // 1
    0x020500, // 2
    0x030642, // 3
    0x040a43, // 4
    0x0500c8, // 5
    0x06c802, // 6
    0x0740b2, // 7
    0x082000, // 8
    0x090604, // 9
    0x0a10d8, // 10
    0x0b0018, // 11
    0x0c5001, // 12
    0x0d4000, // 13
    0x0e1e70, // 14
    0x0f064f, // 15
    0x100080, // 16
    0x110118, // 17
    0x120064, // 18
    0x1327b7, // 19
    0x14d848, // 20
    0x150401, // 21
    0x160001, // 22
    0x17007c, // 23
    0x18071a, // 24
    0x190c2b, // 25
    0x1a0db0, // 26
    0x1b0002, // 27
    0x1c0488, // 28
    0x1d318c, // 29
    0x1e318c, // 30
    0x1f43ec, // 31
    0x200393, // 32
    0x211e21, // 33
    0x220000, // 34
    0x230004, // 35
    0x240800, // 36 0x240800 36.81828 MHz 0x190800 36.82000
    0x250304, // 37
    0x260000, // 38
    0x270001, // 39
    0x280000, // 40
    0x290000, // 41
    0x2a0000, // 42
    0x2b0000, // 43
    0x2c1fa3, // 44
    0x2dc0df, // 45
    0x2e07fc, // 46
    0x2f0300, // 47
    0x300300, // 48
    0x314180, // 49
    0x320000, // 50
    0x330080, // 51
    0x340820, // 52
    0x350000, // 53
    0x360000, // 54
    0x370000, // 55
    0x380000, // 56
    0x390020, // 57
    0x3a8001, // 58
    0x3b0001, // 59
    0x3c0000, // 60
    0x3d00a8, // 61
    0x3e0322, // 62
    0x3f0000, // 63
    0x401388, // 64
    0x410000, // 65
    0x4201f4, // 66
    0x430000, // 67
    0x4403e8, // 68
    0x450000, // 69
    0x46c350, // 70
    0x470081, // 71
    0x480001, // 72
    0x49003f, // 73
    0x4a0000, // 74
    0x4b0b80, // 75
    0x4c000c, // 76
    0x4d0000, // 77
    0x4e00c3, // 78
    0x4f0000, // 79
    0x500000, // 80
    0x510000, // 81
    0x520000, // 82
    0x530000, // 83
    0x540000, // 84
    0x550000, // 85
    0x560000, // 86
    0x570000, // 87
    0x580000, // 88
    0x590000, // 89
    0x5a0000, // 90
    0x5b0000, // 91
    0x5c0000, // 92
    0x5d0000, // 93
    0x5e0000, // 94
    0x5f0000, // 95
    0x600000, // 96
    0x610888, // 97
    0x620000, // 98
    0x630000, // 99
    0x640000, // 100
    0x650011, // 101
    0x660000, // 102
    0x670000, // 103
    0x680000, // 104
    0x690021, // 105
    0x6a0000, // 106
    0x6b0000, // 107
    0x6c0000, // 108
    0x6d0000, // 109
    0x6e0000, // 110
    0x6f0000, // 111
    0x700000, // 112
];

pub static FCAL_EN_OFF: u32 = 0x002414;
pub static FCAL_EN_ON: u32 = REG_MAP[0]; //0x00241c
pub static RESET_ON: u32 = 0x00241e;
pub static RESET_OFF: u32 = REG_MAP[0];

/// Manage the 24-bit registers of the LMX2594
pub trait Lmx2594 {
    /// Return the three bytes of the 24-bit register stored as a u32
    fn reg(&self) -> [u8; 3];
    /// Write the 24-bit register
    fn write_reg(
        &self,
        spi: &mut Spi<Enabled, SPI0, 8>,
        spi_cs: &mut Pin<Gpio5, Output<PushPull>>,
        buf: &mut [u8; 3],
    );
}

// We store the 24-bit register values as u32
impl Lmx2594 for u32 {
    fn reg(&self) -> [u8; 3] {
        let [_, u1, u2, u3] = self.to_be_bytes();
        [u1, u2, u3]
    }

    /// Write register to device. All Results are Infallible
    fn write_reg(
        &self,
        spi: &mut Spi<Enabled, SPI0, 8>,
        spi_cs: &mut Pin<Gpio5, Output<PushPull>>,
        buf: &mut [u8; 3],
    ) {
        spi_cs.set_low().unwrap();
        *buf = self.reg();
        spi.write(buf).unwrap();
        spi_cs.set_high().unwrap();
    }
}
