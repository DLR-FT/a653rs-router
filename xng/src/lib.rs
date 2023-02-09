#![no_std]
#![feature(array_chunks)]
#![feature(let_chains)]

pub mod network;

extern "C" {
    // pub fn XalPrintf(fmt: *const u8, ...) -> i32;
    pub fn XalPutchar(c: i32) -> i32;
}

pub mod logging {
    use core::fmt::{Display, Write};

    use crate::XalPutchar;
    use log::{LevelFilter, Log, Metadata, Record};

    pub struct XalLogger;

    impl Log for XalLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() < log::max_level()
        }

        fn log(&self, record: &Record) {
            let mut writer = XalWriter;
            if let (lvl, Some(file), Some(line)) =
                (record.level(), record.file(), record.line()) && lvl <= LevelFilter::Error
            {
                _ = core::write!(
                    &mut writer,
                    "{}: {} {}: {file} at line {line}\n",
                    record.target(),
                    ColourLogLevel::from(lvl),
                    record.args(),
                );
            } else {
                _ = core::write!(
                    &mut writer,
                    "{}: {} {}\n",
                    record.target(),
                    ColourLogLevel::from(record.level()),
                    record.args(),
                );
            }
        }

        fn flush(&self) {}
    }

    pub struct XalWriter;

    impl Write for XalWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for c in s.chars() {
                unsafe { XalPutchar(c as i32) };
            }
            Ok(())
        }

        fn write_char(&mut self, c: char) -> core::fmt::Result {
            unsafe { XalPutchar(c as i32) };
            Ok(())
        }
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
