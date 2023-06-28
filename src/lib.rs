//! Osiris Platform Integration
//!
//! The osiris platform module integrates rust applications with a wide range
//! of target platforms, including mobile platforms like Android and iOS, as
//! well as desktop platforms like Linux and Windows, or custom platform
//! targets. The platform module is a standalone integration effort, not
//! requiring other osiris modules to be used, nor placing any restrictions
//! on the rust application.
//!
//! The platform module is used to turn a rust application into a proper
//! application for any of the supported target platforms. Platform integration
//! can be under full control of the rust application, allowing direct
//! access to the native application build process of each platform.
//! Alternatively, the platform integration can be left under control of
//! the platform module, thus hiding the entire native integration and instead
//! using the abstractions of the platform module.
//!
//! Model
//! -----
//!
//! The platform module follows an opt-in approach, letting applications choose
//! which part of the platform integration should be directly managed by the
//! application, and which are left to the platform module.
//!
//! On the one end are applications that use the platform module for
//! documentation only, but provide all code and configuration for each target
//! platform in their code-base. The platform module describes the possible
//! options how to integrate rust applications with each target platform, but
//! ultimately, it is up to the application to ship the right code and pull in
//! suitable build tools for artifact assembly.
//!
//! On the other end are applications that leave all platform integration to
//! the platform module. They use the platform module to wrap the rust
//! application suitably for each supported target platform, and use its build
//! suite for final artifact assembly.
//!
//! There are good reasons to be on either end of the spectrum and each
//! application has to decide how much platform-control it requires. Quite
//! likely, applications will start out with little to no platform-specific
//! code, but add more and more platform specifics the closer to release they
//! get, or the more non-portable dependencies they add.
//!
//! The main focus of the platform module is to document how to write rust
//! applications for each supported target platform, and how to build the
//! standard artifacts for each platform. The secondary focus is to provide
//! abstractions over all supported platforms, allowing for automatic
//! artifact assembly for a wide range of target platforms.
//!
//! Platform abstractions include the build-system integration code for
//! artifact assembly, as well as the application code required for runtime
//! integration. The abstractions use the Osiris Platform Manifest for
//! configuration. The manifest is a TOML-formatted file usually called
//! `osiris-platform.toml` placed in the application repository. The
//! `osiris-platform` command-line tool parses the manifest and uses it for
//! all of the platform abstractions. The manifest is required if any of the
//! platform abstractions are used.
//!
//! Supported Platforms
//! -------------------
//!
//! The following list describes the supported target platforms. Each platform
//! is accompanied by documentation on how rust applications can be deployed,
//! and which abstractions are supported by the platform module.
//!
//!  * [Android](platform::android)
//!  * iOS (WIP)
//!  * Linux (WIP)
//!  * MacOS (WIP)
//!  * Web (WIP)
//!  * Windows (WIP)

pub mod manifest;

/// Platform Operations
///
/// The `op` module is a collection of all operations that can be performed via
/// the command-line interface. Each operation is implemented in a submodule
/// and can be used independently.
pub mod op {
    pub mod emerge;
}

/// Platform Integration
///
/// The `platform` module documents how rust applications can be integrated
/// into native applications for each respective platform.
pub mod platform {
    pub mod android;

    /// Platform Identifier
    ///
    /// This enum is an enumeration of supported platforms. It implements
    /// `FromStr` to allow creation from string representation. Use `as_str()`
    /// to get a static string-representation back.
    #[derive(Clone, Copy, Debug)]
    pub enum Id {
        Android,
    }

    impl Id {
        /// Get string representation
        ///
        /// Return the string representation of the platform identifier. This
        /// is guaranteed to be parsable by the `FromStr` implementation.
        pub fn as_str(&self) -> &'static str {
            match self {
                Id::Android => "android",
            }
        }
    }

    // Parse platform identifiers from strings
    //
    // This implements `FromStr` to allow using `std::str::parse()` and thus
    // get platform identifiers from their respective string representation.
    // Note that this uses case-insensitive matching.
    impl std::str::FromStr for Id {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.eq_ignore_ascii_case("android") {
                Ok(Self::Android)
            } else {
                Err(())
            }
        }
    }
}
