//! Platform Manifest
//!
//! This is a rust implementation of the Osiris Platform Manifest Format.
//! Applications can use this manifest to define their platform integration
//! via the osiris platform module.

use serde;
use toml;

/// Raw Manifest Application Table
///
/// Sub-type of `Raw` representing the `Application` table. This contains all
/// configuration regarding the rust application.
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RawApplication {
    /// Path to the application root relative from the manifest.
    pub path: Option<String>,
    /// Human-readable short-name of the application.
    pub name: Option<String>,
    /// Identifier of the application based on the short-name (alphanumeric
    /// with underscores allowed, but not starting with a digit).
    pub id: Option<String>,
    /// Reverse-domain used by the application as namespace.
    pub reverse_domain: Option<String>,
}

/// Android-Platform Table
///
/// Sub-type of `RawPlatform` defining all the Android platform integration
/// options and related definitions.
///
/// The options in this table are one-to-one mappings of their equivalents
/// in the Android Application SDK.
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RawPlatformAndroid {
    pub application_id: Option<String>,
    pub namespace: Option<String>,

    pub compile_sdk: Option<u32>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,

    pub version_code: Option<u32>,
    pub version_name: Option<String>,

    pub sdk_path: Option<String>,
}

/// Raw Manifest Platform Table
///
/// Sub-type of `Raw` representing the `Platform` table. This contains all
/// configuration of the platform integration modules.
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RawPlatform {
    /// Path to the platform integration root relative from the manifest.
    pub path: Option<String>,
    /// Android-platform specific configuration.
    pub android: Option<RawPlatformAndroid>,
}

/// Raw Manifest Content
///
/// This type contains the raw manifest content as parsed by `toml` and
/// converted into rust types via `serde`.
///
/// Note that content of the type is not verified other than for syntactic
/// correctness required by the given types. Semantic correctness needs to
/// be verified by the caller.
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    /// Version of the manifest format. Only version `1` is currently
    /// supported.
    pub version: u32,

    /// Application table specifying properties of the application itself.
    pub application: Option<RawApplication>,
    /// Platform table specifying all properties of the platform integration.
    pub platform: Option<RawPlatform>,
}

impl Raw {
    fn parse_toml(table: toml::Table) -> Result<Self, ()> {
        <Self as serde::Deserialize>::deserialize(table)
            .map_err(|_| ())
    }

    fn parse_str(content: &str) -> Result<Self, ()> {
        content.parse::<toml::Table>()
            .map_err(|_| ())
            .and_then(|v| Self::parse_toml(v))
    }
}

/// Manifest Abstraction
///
/// This type represents a valid and verified manifest. The manifest content
/// can be directly accessed via the `raw` field. The data is verified for
/// semantic correctness (unlike the `Raw` type).
pub struct Manifest {
    /// Raw manifest content as parsed by the TOML module.
    pub raw: Raw,
}

impl Manifest {
    /// Parse manifest from raw
    ///
    /// Take a raw representation of the manifest and perform post-parsing
    /// validation, ensuring the final manifest will not contain invalid
    /// entries.
    fn parse_raw(raw: Raw) -> Result<Self, ()> {
        // We only support version '1'. Any other version number is explicitly
        // defined to be incompatible, so fail parsing.
        //
        // Note that we do support unknown-fields. Hence, it is valid to add
        // more fields to version '1' without breaking backwards compatibility.
        // However, they will be silently ignored by older implementations.
        if raw.version != 1 {
            return Err(());
        }

        // Verify that the application ID, if provided, is a valid identifier.
        // The allowed character-set is alphanumeric and underscores, but not
        // starting with a digit. The full unicode set is allowed.
        if raw.application.as_ref().is_some_and(
            |v| v.id.as_ref().is_some_and(
                |v| {
                    let mut iter = v.chars();
                    let first = iter.next();

                    !(
                        first.is_some_and(|v| v.is_alphabetic() || v == '_')
                        && iter.all(|v| v.is_alphanumeric() || v == '_')
                    )
                }
            )
        ) {
            return Err(());
        }

        Ok(
            Self {
                raw: raw,
            }
        )
    }

    /// Parse manifest from string
    ///
    /// Parse the given string as a literal manifest in TOML representation.
    /// Content is verified and invalid manifests are refused.
    pub fn parse_str(content: &str) -> Result<Self, ()> {
        Raw::parse_str(content)
            .map_err(|_| ())
            .and_then(|v| Self::parse_raw(v))
    }

    /// Parse manifest from file-system
    ///
    /// Open the specified file and parse it as a manifest. The content is
    /// verified and invalid manifests are refused. The file is completely
    /// parsed into memory and then closed again before the function returns.
    pub fn parse_path(path: &std::path::Path) -> Result<Self, ()> {
        std::fs::read_to_string(path)
            .map_err(|_| ())
            .and_then(|v| Self::parse_str(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify basic parsing of `Raw`
    //
    // Parse a minimal raw manifest into `Raw` to have a base-level test for
    // the parsing capabilities. Not complex content verification is done.
    #[test]
    fn raw_parse_minimal() {
        let s = "version = 1";

        Raw::parse_str(s).unwrap();
    }

    // Verify unknown versions in `Raw`
    //
    // Parse a high version number and verify that the raw content parser
    // does not care for its value other than syntactic correctness.
    #[test]
    fn raw_parse_unknown_version() {
        let s = "version = 12345678";

        Raw::parse_str(s).unwrap();
    }

    // Verify basic parsing of `Manifest`
    //
    // Parse a minimal manifest into `Manifest` to have a base-level test for
    // the parsing capabilities. Not complex content verification is done.
    #[test]
    fn manifest_parse_minimal() {
        let s = "version = 1";

        Manifest::parse_str(s).unwrap();
    }

    // Verify parsing of unknown manifest versions
    //
    // Parse an unknown manifest version and verify that the manifest correctly
    // refuses it as invalid.
    #[test]
    fn manifest_parse_unknown_version() {
        let s = "version = 2";

        assert!(Manifest::parse_str(s).is_err());
    }

    // Verify simple parsing of `Manifest`
    //
    // A rather simple parsing test to verify basic sub-field parsing and
    // verification.
    #[test]
    fn manifest_parse_simple() {
        let s = "
            version = 1
            [application]
            path = \".\"
            [platform]
            path = \"./platform\"
        ";

        let m = Manifest::parse_str(s).unwrap();

        assert_eq!(m.raw.version, 1);
        assert_eq!(m.raw.application.unwrap().path.unwrap(), ".");
        assert_eq!(m.raw.platform.unwrap().path.unwrap(), "./platform");
    }

    // Verify parsing of manifest application ids
    //
    // Application IDs use a restrictive character set. Verify the validator
    // and wrong encodings are not allowed.
    #[test]
    fn manifest_parse_application_id() {
        let s = "
            version = 1
            [application]
            id = \"_foobar0\"
        ";

        let m = Manifest::parse_str(s).unwrap();
        assert_eq!(m.raw.application.unwrap().id.unwrap(), "_foobar0");
    }
}
