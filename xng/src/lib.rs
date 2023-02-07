#![no_std]
#![feature(array_chunks)]

pub mod network;

extern "C" {
    pub fn XalPrintf(fmt: *const u8, ...) -> i32;
}

pub mod logging {
    use core::cmp::min;

    use crate::XalPrintf;

    pub struct XalLogger;

    impl log::Log for XalLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() < log::max_level()
        }

        fn log(&self, record: &log::Record) {
            let mut buf = [0u8; 100];
            if let Some(message) = record.args().as_str() {
                {
                    let level = record.level().as_str().as_bytes();
                    let len = min(buf.len() - 3, level.len());
                    buf[0..len].copy_from_slice(level);
                    let end = b": \0";
                    buf[len..len + 3].copy_from_slice(end);
                    unsafe { XalPrintf(buf[0..len].as_ptr()) };
                }

                {
                    let end = b"\n\0";
                    let msg = message.as_bytes();
                    let len = min(buf.len() - 2, msg.len());
                    buf[0..len].copy_from_slice(&msg[0..len]);
                    buf[len..len + 2].copy_from_slice(end);
                    unsafe { XalPrintf(buf[0..len].as_ptr()) };
                }
            } else {
                unsafe { XalPrintf(b"Can not log this...\n\0".as_ptr()) };
            }
        }

        fn flush(&self) {}
    }
}
