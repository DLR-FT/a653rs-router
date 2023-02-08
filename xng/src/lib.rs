#![no_std]
#![feature(array_chunks)]

pub mod network;

extern "C" {
    pub fn XalPrintf(fmt: *const u8, ...) -> i32;
}

pub mod logging {
    use core::cmp::min;

    use crate::XalPrintf;
    use heapless::String;
    use log::{LevelFilter, Log, Metadata, Record};

    pub struct XalLogger;

    impl Log for XalLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() < log::max_level()
        }

        fn log(&self, record: &Record) {
            let mut outstream = String::<200>::new();
            if record.level() <= LevelFilter::Error
                && record.file().is_some()
                && record.line().is_some()
            {
                core::fmt::write(
                    &mut outstream,
                    format_args!(
                        "{}: {} {}: {} at line {}",
                        record.target(),
                        record.level(),
                        record.args(),
                        record.file().unwrap(),
                        record.line().unwrap(),
                    ),
                );
            } else {
                core::fmt::write(
                    &mut outstream,
                    format_args!("{}: {} {}", record.target(), record.level(), record.args(),),
                );
            }

            let outstream = outstream.as_bytes();
            let mut buf = [0u8; 200];
            let len = min(buf.len() - 2, outstream.len());
            buf[0..len].copy_from_slice(outstream);
            let end = b"\n\0";
            buf[len..len + 2].copy_from_slice(end);
            unsafe { XalPrintf(buf.as_ptr()) };
        }

        fn flush(&self) {}
    }
}
