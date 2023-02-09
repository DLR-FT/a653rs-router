#![no_std]
#![feature(array_chunks)]

pub mod network;

extern "C" {
    pub fn XalPrintf(fmt: *const u8, ...) -> i32;
}

pub mod logging {
    use core::{cmp::min, fmt::Display};

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
                _ = core::fmt::write(
                    &mut outstream,
                    format_args!(
                        "{}: {} {}: {} at line {}",
                        record.target(),
                        ColourLogLevel::from(record.level()),
                        record.args(),
                        record.file().unwrap(),
                        record.line().unwrap(),
                    ),
                );
            } else {
                _ = core::fmt::write(
                    &mut outstream,
                    format_args!(
                        "{}: {} {}",
                        record.target(),
                        ColourLogLevel::from(record.level()),
                        record.args(),
                    ),
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

    pub struct ColourLogLevel(log::Level);

    impl ColourLogLevel {
        const RESET_COLOUR: &str = "\x1b[39m";
        const RED_COLOUR: &str = "\x1b[31m";
        const GREEN_COLOUR: &str = "\x1b[32m";
        const YELLOW_COLOUR: &str = "\x1b[33m";
        const BLUE_COLOUR: &str = "\x1b[34m";
        const MAGENTA_COLOUR: &str = "\x1b[35m";

        pub fn as_colour_code(&self) -> &'static str {
            match self.0 {
                log::Level::Error => Self::RED_COLOUR,
                log::Level::Warn => Self::YELLOW_COLOUR,
                log::Level::Info => Self::GREEN_COLOUR,
                log::Level::Debug => Self::BLUE_COLOUR,
                log::Level::Trace => Self::MAGENTA_COLOUR,
            }
        }
    }

    impl From<log::Level> for ColourLogLevel {
        fn from(level: log::Level) -> Self {
            Self(level)
        }
    }

    impl Display for ColourLogLevel {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(
                f,
                "{}{}{}",
                self.as_colour_code(),
                self.0,
                Self::RESET_COLOUR
            )
        }
    }
}
