//! A small tracer that writes 8 bit GPIO. See <https://docs.xilinx.com/v/u/en-US/pg144-axi-gpi>
//!
//! Only the first of two GPIO channels is enabled. It is set to output only
//! with a width of 8 bit. The bits are mapped to ports IO 26 to 33 on the
//! CoraZ7.
//!
//! The fpga design files are at <https://gitlab.dlr.de/projekt-resilienz/vivado-coraz7-uart>

use core::marker::{Send, Sync};
use small_trace::{TraceEvent, Tracer};
use volatile_register::{RO, RW};

/// From board config
///  "SEG_axi_gpio_0_Reg": {
///      "address_block": "/axi_gpio_0/S_AXI/Reg",
///      "offset": "0x7FFF_8000",
///      "range": "32K"
///  },
const REG_AXI_GPIO: usize = 0x8000_0000;

/// Starts at SLCR_BASE + MIO_PIN_00
#[repr(C)]
struct AxiGpioRegs {
    pub gpio_data: RW<u32>,
    pub gpio_tri: RW<u32>,
    pub gpio2_data: RW<u32>, // not configured in FPGA design
    pub gpio2_tri: RW<u32>,  // not configured in FPGA design
    pub _unassigned: [RO<u8>; 0x10f],
    pub gier: RW<u32>, // 0x11C
    pub ip_isr: RW<u32>,
    pub ip_ier: RW<u32>,
}

pub struct GpioTracer {
    gpio: *const AxiGpioRegs,
}

impl GpioTracer {
    pub const fn new() -> Self {
        Self {
            gpio: REG_AXI_GPIO as *const AxiGpioRegs,
        }
    }

    /// Initializes the GPIOs by writing the reset value to the control
    /// registers.
    pub fn init(&self) {
        self.set_output(0x0);
    }

    /// Writes a value to GPIO pins 0..8
    pub fn write(&self, val: u32) {
        unsafe { (*self.gpio).gpio_data.write(val) };
        // unsafe { (*self.gpio).gpio_data.write(0x0) }
    }

    fn set_output(&self, mask: u32) {
        unsafe { (*self.gpio).gpio_tri.write(mask) };
    }
}

impl Default for GpioTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracer for GpioTracer {
    /// Writes the bits of the value to IO pins 26 to 41. Then resets the traced
    /// value to 0.
    fn trace(&self, val: TraceEvent) {
        self.write(u16::from(val) as u32);
    }
}

// This is fine...
unsafe impl Send for GpioTracer {}
unsafe impl Sync for GpioTracer {}
