use one_byte_trace::Tracer;
use volatile_register::RW;

const SLCR_BASE: usize = 0xF800_0000;
const MIO_PIN_00: usize = 0x700;

const GPIO_BASE: usize = 0xE000_A000;
const XGPIOPS_DIRM_OFFSET: usize = 0x204;

const MIO_PIN_XX_RESET: u32 = 0x1601;

/// Starts at SLCR_BASE + MIO_PIN_00
#[repr(C)]
struct SlcrMio {
    pub mio_pins: [RW<u32>; 53],
}

#[repr(C)]
struct GpioCtlRegister {
    /// XGPIOPS_DIRM_OFFSET             0x00000204 32    rw    0x00000000  Direction mode (GPIO Bank0, MIO)
    pub dirm_0: RW<u32>,
    /// XGPIOPS_OUTEN_OFFSET            0x00000208 32    rw    0x00000000  Output enable (GPIO Bank0, MIO)
    pub outen_0: RW<u32>,
}

#[repr(C)]
struct GpioDataRegister {
    /// XGPIOPS_DATA_LSW_OFFSET         0x00000000 32    mixed x           Maskable Output Data (GPIO Bank0, MIO, Lower 16bits)
    pub mask_data_0_lsw: RW<u32>,
    /// XGPIOPS_DATA_MSW_OFFSET         0x00000004 32    mixed x           Maskable Output Data (GPIO Bank0, MIO, Upper 16bits)
    pub mask_data_0_msw: RW<u32>,
}

pub struct GpioTracer {
    slcr: *const SlcrMio,
    gpio_ctl: *const GpioCtlRegister,
    gpio_data: *const GpioDataRegister,
}

impl GpioTracer {
    pub const fn new() -> Self {
        Self {
            slcr: (SLCR_BASE + MIO_PIN_00) as *const SlcrMio,
            gpio_ctl: (GPIO_BASE + XGPIOPS_DIRM_OFFSET) as *const GpioCtlRegister,
            gpio_data: GPIO_BASE as *const GpioDataRegister,
        }
    }

    /// Initializes the GPIOs by writing the reset value to the control registers.
    pub fn init(&self) {
        for i in 0..8 {
            self.write_mio(i, MIO_PIN_XX_RESET)
        }
        // Set pins 0..8 to as outputs.
        self.set_direction(0xFF);

        // Enable output on pins 0..8
        self.enable_output(0xFF)
    }

    /// Writes a value to GPIO pins 0..8
    pub fn write(&self, val: u8) {
        // Always change all bytes
        //let lsw_mask: u16 = 0xffff - val;
        let out = val as u32;
        unsafe { (*self.gpio_data).mask_data_0_lsw.write(out) }
    }

    fn enable_output(&self, mask: u32) {
        unsafe { (*self.gpio_ctl).outen_0.write(mask) }
    }

    fn set_direction(&self, mask: u32) {
        unsafe { (*self.gpio_ctl).dirm_0.write(mask) }
    }

    fn write_mio(&self, mio: usize, val: u32) {
        unsafe { (*self.slcr).mio_pins[mio].write(val) }
    }
}

impl Tracer for GpioTracer {
    /// Writes the bits of the value to IO pins 0 to 7. Then resets the traced value to 0.
    fn trace(&self, val: u8) {
        self.write(val);
        self.write(0x0)
    }
}

// This is fine...
unsafe impl Send for GpioTracer {}
unsafe impl Sync for GpioTracer {}
