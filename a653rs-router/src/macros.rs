#[cfg(not(feature = "log"))]
macro_rules! router_log {
    ($level:ident, $($arg:expr),*) => {{ $( let _ = $arg; )* }}
}

#[cfg(all(feature = "log", test))]
macro_rules! router_log {
    ($level:ident, $($arg:expr),*) => {{ $( let _ = $arg; )* }}

}

#[cfg(all(feature = "log", not(test)))]
macro_rules! router_log {
    (trace, $($arg:expr),*) => { log::trace!($($arg),*) };
    (debug, $($arg:expr),*) => { log::debug!($($arg),*) };
}

macro_rules! router_trace {
    ($($arg:expr),*) => (router_log!(trace, $($arg),*));
}

macro_rules! router_debug {
    ($($arg:expr),*) => (router_log!(debug, $($arg),*));
}

#[cfg(not(test))]
#[cfg(feature = "trace")]
macro_rules! router_bench {
    ($arg0:ident) => {
        small_trace::small_trace!($arg0)
    };
    ($arg0:ident, $arg1:expr) => {
        small_trace::small_trace!($arg0, $arg1)
    };
}

#[cfg(any(test, not(feature = "trace")))]
macro_rules! router_bench {
    ($arg0:ident) => {
        let _ = "";
    };
    ($arg0:ident, $arg1:expr) => {
        let _ = $arg1;
    };
}
