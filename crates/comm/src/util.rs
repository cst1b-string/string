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
}

/// A convenient macro for breaking out of a loop if an error occurs, as well
/// as printing a debug message.
#[macro_export]
macro_rules! try_break_debug {
    ($e:expr, $msg:literal) => {
        match $e {
            Ok(e) => e,
            Err(_) => {
                debug!($msg)
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
}

/// A convenient macro for skipping this loop iteration if an error occurs, as well
/// as printing a debug message.
#[macro_export]
macro_rules! try_continue_debug {
    ($e:expr, $msg:literal) => {
        match $e {
            Ok(e) => e,
            Err(_) => {
                debug!($msg);
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
}

/// A convenient macro for breaking out of a loop if a value if None, as well
/// as printing a debug message.
#[macro_export]
macro_rules! maybe_break_debug {
    ($e:expr, $msg:literal) => {
        match $e {
            Some(e) => e,
            None => {
                debug!($msg)
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
}

/// A convenient macro for skipping this loop iteration if a value if None, as well
/// as printing a debug message.
#[macro_export]
macro_rules! maybe_continue_debug {
    ($e:expr, $msg:literal) => {
        match $e {
            Some(e) => e,
            None => {
                debug!($msg);
                continue;
            }
        }
    };
}
