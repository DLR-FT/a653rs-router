#[cfg(not(test))]
#[cfg(feature = "log")]
macro_rules! router_log {
    (trace, $($arg:expr),*) => { log::trace!($($arg),*) };
    (debug, $($arg:expr),*) => { log::debug!($($arg),*) };
}

#[cfg(test)]
#[cfg(feature = "log")]
macro_rules! router_log {
    (trace, $($arg:expr),*) => { println!($($arg),*) };
    (debug, $($arg:expr),*) => { println!($($arg),*) };
}

#[cfg(not(feature = "log"))]
macro_rules! router_log {
    ($level:ident, $($arg:expr),*) => {{ $( let _ = $arg; )* }}
}

macro_rules! router_trace {
    ($($arg:expr),*) => (router_log!(trace, $($arg),*));
}

macro_rules! router_debug {
    ($($arg:expr),*) => (router_log!(debug, $($arg),*));
}
