use a653rs_router::prelude::{
    CreateNetworkInterfaceId, InterfaceConfig, InterfaceError, NetworkInterfaceId,
    PlatformNetworkInterface,
};
use cobs::{decode_in_place, encode};
use core::mem::size_of;
use heapless::spsc::Queue;
use once_cell::unsync::Lazy;
use uart_xilinx::MmioUartAxi16550;

/// Networking on XNG.
#[derive(Debug)]
pub struct UartNetworkInterface<const MTU: usize>;

mod config {
    pub const BASE_ADDRESS: usize = 0x43C0_0000;
    pub const CLOCK_RATE: usize = 50_000_000;
    pub const BAUD_RATE: usize = 115200;
    pub const FIFO_DEPTH: usize = 16;
    pub const FRAME_BUFFER: usize = 20000;
}

struct UartFrame<'p, const MTU: usize> {
    pl: &'p [u8],
}

/// encoded: COBS(vl_id + payload + CRC16)
impl<'p, const MTU: usize> UartFrame<'p, MTU> {
    const fn max_decoded_len() -> usize {
        core::mem::size_of::<u16>() + MTU + core::mem::size_of::<u16>()
    }

    const fn max_encoded_len() -> usize {
        max_encoded_len(2 * core::mem::size_of::<u16>() + MTU) + 1
    }

    fn frame_encoded_len(&self) -> usize {
        max_encoded_len(core::mem::size_of::<u16>() + self.pl.len() + core::mem::size_of::<u16>())
    }

    /// Encodes the frame contents, excluding the
    fn encode<'a>(&self, encoded: &'a mut [u8]) -> Result<&'a [u8], ()>
    where
        [(); Self::max_decoded_len()]:,
    {
        let mut buf = [0u8; Self::max_decoded_len()];
        if self.pl.len() > MTU || self.frame_encoded_len() > encoded.len() {
            return Err(());
        }

        // Payload
        buf[..self.pl.len()].copy_from_slice(self.pl);

        // CRC
        let crc = crc16::State::<crc16::USB>::calculate(&buf[..self.pl.len()]);
        let crc: [u8; 2] = crc.to_be_bytes();
        buf[self.pl.len()..self.pl.len() + 2].copy_from_slice(&crc);

        // COBS encode
        let enclen = encode(&buf[0..self.pl.len() + 2], encoded);

        Ok(&encoded[..enclen])
    }

    fn decode(buf: &mut [u8]) -> Result<&[u8], ()> {
        // COBS decode
        let declen = decode_in_place(buf).or(Err(()))?;

        let crclen = size_of::<u16>();
        if declen < crclen {
            return Err(());
        }

        // Check CRC
        let (msg, crc) = buf.split_at(declen - crclen);
        let crc = crc[0..2].try_into().or(Err(()))?;
        let rcrc = u16::from_be_bytes(crc);
        let crc = crc16::State::<crc16::USB>::calculate(msg);
        if rcrc != crc {
            return Err(());
        }

        Ok(msg)
    }
}

struct BufferedUart<const BUFFER_LEN: usize> {
    uart: MmioUartAxi16550<'static>,
    rx_buffer: Queue<u8, BUFFER_LEN>,
}

impl<const BUFFER_LEN: usize> BufferedUart<BUFFER_LEN> {
    fn new(base_address: usize) -> Self {
        BufferedUart {
            uart: MmioUartAxi16550::new(base_address),
            rx_buffer: Queue::new(),
        }
    }

    #[allow(clippy::unusual_byte_groupings)]
    fn init(&mut self, clock_rate: usize, baud_rate: usize) {
        // Disable interrupts
        self.uart.write_ier(0);

        _ = self.uart.read_ier();

        // Change in DCDN after last MSR read.
        _ = self.uart.read_msr();

        // Reset line status?
        _ = self.uart.read_lsr();

        // Reset modem control register.
        self.uart.write_mcr(0);

        // Sets clock divisor and baud rate. Enables interrupts.
        self.uart.init(clock_rate, baud_rate);

        // Disables interrupts. Use polling instead.
        self.uart.write_ier(0);

        // Use one stop bit, no parity bit, 8 bit/character.
        // We have CRC for each frame, so parity is redundant.
        // 7 DLAB, 6 Set Break, 5 Stick Partity, 4 EPS, 3 PEN, 2 STB, 1-0 WLS
        self.uart.write_lcr(0b0_0_0_0_0_0_11);

        // FIFO has 16 character (byte?) length
        // Rx FIFO trigger level is 1 byte, reset Rx & Tx FIFO, enable FIFO
        self.uart.write_fcr(0b00_000_11_1);
    }
}

impl<const BUFFER_LEN: usize> Drop for BufferedUart<BUFFER_LEN> {
    fn drop(&mut self) {
        self.uart.write_ier(0);
        _ = self.uart.read_msr();
        _ = self.uart.read_lsr();
        // Reset RX and TX FIFO, disable FIFO
        #[allow(clippy::unusual_byte_groupings)]
        self.uart.write_fcr(0b00_000_11_0);
    }
}

static mut UART: Lazy<BufferedUart<{ config::FRAME_BUFFER }>> = Lazy::new(|| {
    let mut b = BufferedUart::new(config::BASE_ADDRESS);
    b.init(config::CLOCK_RATE, config::BAUD_RATE);
    b
});

impl<const MTU: usize> PlatformNetworkInterface for UartNetworkInterface<MTU>
where
    [(); UartFrame::<MTU>::max_encoded_len()]:,
    [(); UartFrame::<MTU>::max_decoded_len()]:,
{
    fn platform_interface_receive_unchecked(
        id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<&'_ [u8], InterfaceError> {
        if unsafe { !UART.uart.is_data_ready() } {
            return Err(InterfaceError::NoData);
        }
        trace!(begin_network_receive, id.0 as u16);
        let mut limit = 0;
        let mut queue_has_eof = false;
        while limit < u8::MAX && !queue_has_eof {
            limit += 1;
            while let Some(b) = unsafe { UART.uart.read_byte() } {
                _ = unsafe { UART.rx_buffer.enqueue(b) };
                if b == 0x0 {
                    queue_has_eof = true;
                    break;
                }
            }
        }
        if !queue_has_eof {
            trace!(end_network_receive, id.0 as u16);
            return Err(InterfaceError::NoData);
        }
        let mut buf = [0u8; UartFrame::<MTU>::max_encoded_len()];
        for b in buf.iter_mut() {
            if let Some(c) = unsafe { UART.rx_buffer.dequeue() } {
                *b = c;
                if c == 0x0 {
                    break;
                }
            } else {
                break;
            }
        }
        match UartFrame::<MTU>::decode(&mut buf) {
            Ok(pl) => {
                let rpl = &mut buffer[0..pl.len()];
                rpl.copy_from_slice(pl);
                trace!(end_network_receive, id.0 as u16);
                Ok(rpl)
            }
            _ => {
                trace!(end_network_receive, id.0 as u16);
                Err(InterfaceError::InvalidData)
            }
        }
    }

    fn platform_interface_send_unchecked(
        id: NetworkInterfaceId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError> {
        let mut buf = [0u8; UartFrame::<MTU>::max_encoded_len()];
        let frame = UartFrame::<MTU> { pl: buffer };

        let encoded =
            UartFrame::<MTU>::encode(&frame, &mut buf).or(Err(InterfaceError::InvalidData))?;

        trace!(begin_network_send, id.0 as u16);
        unsafe {
            let mut index: usize = 0;
            while index < encoded.len() {
                while !UART.uart.is_transmitter_holding_register_empty() {}
                for _ in 0..config::FIFO_DEPTH {
                    UART.uart.write_byte(encoded[index]);
                    index += 1;
                    if index == encoded.len() {
                        break;
                    }
                }
            }

            // Wait for transmission to finish
            while !UART.uart.is_transmitter_holding_register_empty() {}
        }
        trace!(end_network_send, id.0 as u16);

        Ok(encoded.len())
    }
}

static mut ID: usize = 0;

fn next_id() -> NetworkInterfaceId {
    unsafe {
        ID += 1;
        NetworkInterfaceId::from(ID)
    }
}

impl<const MTU: usize> CreateNetworkInterfaceId<UartNetworkInterface<MTU>>
    for UartNetworkInterface<MTU>
where
    [(); UartFrame::<MTU>::max_encoded_len()]:,
    [(); UartFrame::<MTU>::max_decoded_len()]:,
{
    fn create_network_interface_id(
        _cfg: &InterfaceConfig,
    ) -> Result<NetworkInterfaceId, InterfaceError> {
        Ok(next_id())
    }
}

const fn max_encoded_len(raw_len: usize) -> usize {
    let overhead = if raw_len == 0 {
        // Just 0xff
        1
    } else {
        (raw_len + 253) / 254
    };
    // +1 for terminator byte.
    raw_len + overhead + 1
}
