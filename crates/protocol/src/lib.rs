//! # string-protocol
//!
//! This crate contains the protocol definition for the string protocol.

/// Utility macro to quickly define a module for a protocol.
macro_rules! include_protocol {
    ($name:literal, $version:ident) => {
        #[doc=concat!("Documentation for version", stringify!($version), "of the", $name, "protocol.")]
        pub mod $version {
            include!(concat!(
                env!("OUT_DIR"),
                "/string.",
                $name,
                ".",
                stringify!($version),
                ".rs",
            ));
        }
    };
}

/// Defines the user buffer types and data.
pub mod users {
    include_protocol!("users", v1);
}

/// Defines the messages buffer types and data.
pub mod messages {
    include_protocol!("messages", v1);
}
