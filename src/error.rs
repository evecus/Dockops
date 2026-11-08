use std::result;

/// Every fallible internal operation returns a human readable error string,
/// mirroring Go's `error.Error()` which is passed straight to the HTTP response.
pub type Result<T> = result::Result<T, String>;

/// Minimal `log.Printf`-style logging to stderr.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        eprintln!(
            "[{}] {}",
            chrono::Local::now().format("%Y/%m/%d %H:%M:%S"),
            format_args!($($arg)*)
        );
    }};
}
