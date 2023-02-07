#![no_std]
#![feature(array_chunks)]

pub mod network;

extern "C" {
    pub fn XalPrintf(fmt: *const u8, ...) -> i32;
}

pub mod logging {
    use crate::XalPrintf;

    pub struct XalLogger;

    impl log::Log for XalLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() < log::max_level()
        }

        fn log(&self, record: &log::Record) {
            unsafe { XalPrintf(b"AAAAAAAAA\n\0".as_ptr()) };

            // if self.enabled(record.metadata()) {
            // let mut buf = [0u8; 100];
            // let level = record.level();
            //if let Some(args) = record.args().as_str() {
            //let mut i = 0usize;
            // for (b, a) in buf[..98].iter_mut().zip(args.as_bytes()) {
            //     *b = *a;
            //     i += 1;
            // }
            //buf[i] = 0x0;
            //unsafe { XalPrintf(b"%s\n\0".as_ptr(), buf.as_ptr()) };
            //}
            //unsafe { XalPrintf(b"[%s] - [%s]\n\0".as_ptr(), level.as_str().char_indices };
            // }
        }

        fn flush(&self) {}
    }
}
