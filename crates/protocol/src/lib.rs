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
                "/str.",
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

/// Defines the crypto buffer types and data.
pub mod crypto {
    include_protocol!("crypto", v1);
}

/// Defines the channel buffer types and data.
pub mod channels {
    include_protocol!("channels", v1);
}

/// Defines the network buffer types and data.
pub mod network {
    include_protocol!("network", v1);
}
