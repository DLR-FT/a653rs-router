use core::time::Duration;

use corncobs::{decode_in_place, encode_buf, max_encoded_len};
use heapless::spsc::Queue;
use network_partition::prelude::{
    CreateNetworkInterfaceId, Error, NetworkInterfaceId, PlatformNetworkInterface, VirtualLinkId,
};
use once_cell::unsync::Lazy;
use uart_xilinx::MmioUartAxi16550;

use crate::XalPrintf;

/// Networking on XNG.
#[derive(Debug)]
pub struct UartSerial;

mod config {
    pub const BASE_ADDRESS: usize = 0x43C0_0000; // TODO
    pub const CLOCK_RATE: usize = 100_000_000; // TODO
    pub const BAUD_RATE: usize = 921600; // TODO
    pub const MTU: usize = 100;
    pub const FRAME_BUFFER_LEN: usize = 2 * (2 + 2 + MTU);
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

        if declen < 2 * core::mem::size_of::<u16>() {
            return Err(());
        }

        // Check CRC
        let (msg, crc) = buf.split_at(declen - 2);
        let rcrc = u16::from_be_bytes(crc.try_into().or(Err(()))?);
        let crc = crc16::State::<crc16::USB>::calculate(msg);
        if rcrc != crc {
            return Err(());
        }

        // VL ID
        let (vl, _) = buf.split_at(2);
        let vl: [u8; 2] = vl.try_into().or(Err(()))?;
        let vl = u16::from_be_bytes(vl);

        // Payload
        let pl = &buf[2..declen - 2];

        Ok((VirtualLinkId::from_u32(vl as u32), pl))
    }
}

struct BufferedUart<const BUFFER_LEN: usize> {
    uart: MmioUartAxi16550<'static>,
    rx_buffer: Queue<u8, BUFFER_LEN>,
    tx_buffer: Queue<u8, BUFFER_LEN>,
}

impl<const BUFFER_LEN: usize> BufferedUart<BUFFER_LEN> {
    const FIFO_DEPTH: usize = 16;

    fn new(base_address: usize) -> Self {
        BufferedUart {
            uart: MmioUartAxi16550::new(base_address),
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

        // TODO FIFO introduces delay. Is FIFO required for performance?
        // FIFO has 16 character (byte?) length
        // Rx FIFO trigger level=14, reset Rx & Tx FIFO, enable FIFO
        self.uart.write_fcr(0b11_000_11_1);
    }

    fn try_write(&mut self) {
        if self.uart.is_transmitter_holding_register_empty() {
            for _ in 0..Self::FIFO_DEPTH {
                if let Some(b) = self.tx_buffer.dequeue() {
                    self.uart.write_byte(b)
                }
            }
        }
    }

    fn try_read(&mut self) {
        if self.rx_buffer.len() == self.rx_buffer.capacity() {
            return;
        }
        while let Some(b) = self.uart.read_byte() {
            // Check if there is space for 1 more byte
            _ = self.rx_buffer.enqueue(b);
        }
    }

    fn has_frame(&self) -> bool {
        self.rx_buffer.iter().any(|b| *b == 0x0)
    }

    fn transmission_duration(&self, bytes: usize) -> Duration {
        // TODO Calculate how long the transmission should take...
        Duration::from_micros(bytes as u64)
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
    ) -> Result<(VirtualLinkId, &'_ [u8]), Error> {
        // TODO Get rid of one buffer. Should be possible to decode directly inside RX-Buffer.
        unsafe { XalPrintf(b"platform_interface_receive_unchecked\n\0".as_ptr()) };
        let mut buf = [0u8; { UartFrame::max_encoded_len() + 1 }];
        if unsafe { !UART.has_frame() } {
            return Err(Error::InterfaceReceiveFail);
        }
        let mut end = 0;
        #[allow(clippy::needless_range_loop)]
        for i in 0..buf.len() {
            if let Some(b) = unsafe { UART.rx_buffer.dequeue() } {
                buf[i] = b;
                // Check for EOF
                if b == 0x0 {
                    end = i;
                    break;
                }
            } else {
                // The RX-buffer has no more elements, but we checked,
                // so it should have at least one full frame.
                return Err(Error::InterfaceReceiveFail);
            }
        }
        let buf = &mut buf[..end];
        match UartFrame::decode(buf) {
            Ok((vl, pl)) => {
                let rpl = &mut buffer[0..pl.len()];
                rpl.copy_from_slice(pl);
                Ok((vl, rpl))
            }
            _ => Err(Error::InterfaceReceiveFail),
        }
    }

    fn platform_interface_send_unchecked(
        _id: NetworkInterfaceId,
        vl: VirtualLinkId,
        buffer: &[u8],
    ) -> Result<Duration, Duration> {
        unsafe { XalPrintf(b"platform_interface_send_unchecked\n\0".as_ptr()) };
        // TODO Get rid of one buffer. Should be possible to encode directly inside TX-Buffer.
        let mut buf = [0u8; { UartFrame::max_encoded_len() + 1 }];
        let frame = UartFrame { vl, pl: buffer };
        // Time it takes to do this should be accounted for if line is not used.
        // TODO encode using iterator for more performance
        let encoded = UartFrame::encode(&frame, &mut buf).or(Err(Duration::ZERO))?;
        if unsafe { UART.tx_buffer.capacity() - UART.tx_buffer.len() < encoded.len() } {
            return Err(Duration::ZERO); // TODO InterfaceSendFail
        }

        // TODO measure transmission time, cancel if time limit exceeded

        Ok(unsafe { UART.transmission_duration(buffer.len()) })
    }
}

impl<H: PlatformNetworkInterface> CreateNetworkInterfaceId<H> for UartSerial {
    fn create_network_interface_id(
        _name: &str, // TODO use network_partition_config::config::InterfaceName ?
        _destination: &str,
        _rate: network_partition::prelude::DataRate,
    ) -> Result<NetworkInterfaceId, Error> {
        todo!()
    }
}
