use core::mem::size_of;

use corncobs::{decode_in_place, encode_buf, max_encoded_len};
use heapless::spsc::Queue;
use log::error;
use network_partition::prelude::{
    CreateNetworkInterfaceId, InterfaceError, NetworkInterfaceId, PlatformNetworkInterface,
    VirtualLinkId,
};
use once_cell::unsync::Lazy;
use uart_xilinx::MmioUartAxi16550;

/// Networking on XNG.
#[derive(Debug)]
pub struct UartSerial;

mod config {
    pub const BASE_ADDRESS: usize = 0x43C0_0000;
    pub const CLOCK_RATE: usize = 50_000_000;
    pub const BAUD_RATE: usize = 115200;
    pub const MTU: usize = 100;
    pub const FRAME_BUFFER_LEN: usize = 1000;
    pub const FIFO_DEPTH: usize = 16;
}

struct UartFrame<'p> {
    vl: VirtualLinkId,
    pl: &'p [u8],
}

/// encoded: COBS(vl_id + payload + CRC16)
impl<'p> UartFrame<'p> {
    const fn max_decoded_len() -> usize {
        core::mem::size_of::<u16>() + config::MTU + core::mem::size_of::<u16>()
    }

    const fn max_encoded_len() -> usize {
        max_encoded_len(2 * core::mem::size_of::<u16>() + config::MTU)
    }

    fn frame_encoded_len(&self) -> usize {
        max_encoded_len(core::mem::size_of::<u16>() + self.pl.len() + core::mem::size_of::<u16>())
    }

    /// Encodes the frame contents, excluding the
    fn encode<'a>(&self, encoded: &'a mut [u8]) -> Result<&'a [u8], ()> {
        let mut buf = [0u8; Self::max_decoded_len()];
        if self.pl.len() > config::MTU || self.frame_encoded_len() > encoded.len() {
            return Err(());
        }

        // VL ID
        let vl_id: [u8; 2] = (self.vl.into_inner() as u16).to_be_bytes();
        buf[0..2].copy_from_slice(&vl_id);

        // Payload
        buf[2..self.pl.len() + 2].copy_from_slice(self.pl);

        // CRC
        let crc = crc16::State::<crc16::USB>::calculate(&buf[..self.pl.len() + 2]);
        let crc: [u8; 2] = crc.to_be_bytes();
        buf[self.pl.len() + 2..self.pl.len() + 4].copy_from_slice(&crc);

        // COBS encode
        let enclen = encode_buf(&buf[0..self.pl.len() + 4], encoded);

        Ok(&encoded[..enclen])
    }

    fn decode(buf: &mut [u8]) -> Result<(VirtualLinkId, &[u8]), ()> {
        // COBS decode
        let declen = decode_in_place(buf).or(Err(()))?;

        let crclen = size_of::<u16>();
        if declen < crclen {
            return Err(());
        }

        // Check CRC
        let (msg, crc) = buf.split_at(declen - crclen);
        let rcrc = u16::from_be_bytes(crc.try_into().or(Err(()))?);
        let crc = crc16::State::<crc16::USB>::calculate(msg);
        if rcrc != crc {
            return Err(());
        }

        // VL ID
        let (vl, pl) = buf.split_at(2);
        let vl: [u8; 2] = vl.try_into().or(Err(()))?;
        let vl = u16::from_be_bytes(vl);

        Ok((VirtualLinkId::from_u32(vl as u32), pl))
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

static mut UART: Lazy<BufferedUart<{ config::FRAME_BUFFER_LEN }>> = Lazy::new(|| {
    let mut b = BufferedUart::new(config::BASE_ADDRESS);
    b.init(config::CLOCK_RATE, config::BAUD_RATE);
    b
});

impl PlatformNetworkInterface for UartSerial {
    fn platform_interface_receive_unchecked(
        _id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<(VirtualLinkId, &'_ [u8]), InterfaceError> {
        if unsafe { !UART.uart.is_data_ready() } {
            return Err(InterfaceError::NoData);
        }
        // TODO Get rid of one buffer. Should be possible to decode directly inside RX-Buffer.
        let mut limit = 0;
        let mut queue_has_eof = false;
        while limit < u128::MAX && !queue_has_eof {
            while let Some(b) = unsafe { UART.uart.read_byte() } {
                limit += 1;

                _ = unsafe { UART.rx_buffer.enqueue(b) };
                if b == 0x0 {
                    queue_has_eof = true;
                    break;
                }
            }
        }
        if !queue_has_eof {
            return Err(InterfaceError::NoData);
        }
        let mut buf = [0u8; { UartFrame::max_encoded_len() + 1 }];
        let mut end = 0;
        for b in buf.iter_mut() {
            if let Some(c) = unsafe { UART.rx_buffer.dequeue() } {
                *b = c;
                end += 1;
                if c == 0x0 {
                    break;
                }
            } else {
                break;
            }
        }
        let buf = &mut buf[..end];
        error!("Received data: {:?}", buf);
        match UartFrame::decode(buf) {
            Ok((vl, pl)) => {
                let rpl = &mut buffer[0..pl.len()];
                rpl.copy_from_slice(pl);
                Ok((vl, rpl))
            }
            _ => Err(InterfaceError::InvalidData),
        }
    }

    fn platform_interface_send_unchecked(
        _id: NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError> {
        let mut buf = [0u8; { UartFrame::max_encoded_len() + 1 }];
        let frame = UartFrame { vl, pl: buffer };

        // TODO Time it takes to do this should be accounted for if line is not used.
        let encoded = UartFrame::encode(&frame, &mut buf).or(Err(InterfaceError::InvalidData))?;

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

        Ok(encoded.len())
    }

    fn platform_interface_get_encoded_len(
        _id: NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError> {
        // It's ugly to encode things twice, because it uses more CPU resources.
        // On the other hand, the not-encoded frame saves some memory when storing it inside the queue.
        let mut buf = [0u8; { UartFrame::max_encoded_len() + 1 }];
        let frame = UartFrame { vl, pl: buffer };

        // TODO Time it takes to do this should be accounted for if line is not used.
        match UartFrame::encode(&frame, &mut buf).or(Err(InterfaceError::InvalidData)) {
            Ok(encoded) => Ok(encoded.len()),
            Err(_) => Err(InterfaceError::InvalidData),
        }
    }
}

impl<H: PlatformNetworkInterface> CreateNetworkInterfaceId<H> for UartSerial {
    fn create_network_interface_id(
        _name: &str, // TODO use network_partition_config::config::InterfaceName ?
        _destination: &str,
        _rate: network_partition::prelude::DataRate,
    ) -> Result<NetworkInterfaceId, InterfaceError> {
        Ok(NetworkInterfaceId(1))
    }
}
