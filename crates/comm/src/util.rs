//! This module contains utilities for use throughout the rest of the codebase.

/// A convenient macro for breaking out of a loop if an error occurs.
#[macro_export]
macro_rules! try_break {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(_) => break,
        }
    };
    ($e:expr,  $msg:literal) => {
        match $e {
            Ok(e) => e,
            Err(err) => {
                error!(concat!($msg, ": {:?}"), err);
                break;
            }
        }
    };
}

/// A convenient macro for skipping this loop iteration if an error occurs.
#[macro_export]
macro_rules! try_continue {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(_) => continue,
        }
    };
    ($e:expr, $msg:literal) => {
        match $e {
            Ok(e) => e,
            Err(err) => {
                error!(concat!($msg, ": {:?}"), err);
                continue;
            }
        }
    };
}

/// A convenient macro for breaking out of a loop if a value if None.
#[macro_export]
macro_rules! maybe_break {
    ($e:expr) => {
        match $e {
            Some(e) => e,
            None => break,
        }
    };
    ($e:expr, $($arg:tt)*) => {
        match $e {
            Some(e) => e,
            None => {
                error!($($arg)*);
                break;
            }
        }
    };
}

/// A convenient macro for skipping this loop iteration if a value if None.
#[macro_export]
macro_rules! maybe_continue {
    ($e:expr) => {
        match $e {
            Some(e) => e,
            None => continue,
        }
    };
    ($e:expr, $($arg:tt)*) => {
        match $e {
            Some(e) => e,
            None => {
                error!($($arg)*);
                break;
            }
        }
    };
}
