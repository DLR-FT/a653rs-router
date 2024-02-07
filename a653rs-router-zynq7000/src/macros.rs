#[cfg(not(test))]
#[cfg(feature = "trace")]
macro_rules! trace {
    ($arg0:ident) => {
        small_trace::small_trace!($arg0)
    };
    ($arg0:ident, $arg1:expr) => {
        small_trace::small_trace!($arg0, $arg1)
    };
}

#[cfg(any(test, not(feature = "trace")))]
macro_rules! trace {
    ($arg0:ident) => {
        let _ = "";
    };
    ($arg0:ident, $arg1:expr) => {
        let _ = $arg1;
    };
}
