//! Osiris Platform Tooling
//!
//! This is the entry-point of `osiris-platform`, a command-line tool to
//! control the platform-integration of rust applications. Its main input is
//! the `osiris-platform.toml` manifest, which specifies parameters of an
//! application and its platform integration. This tool reads the manifest
//! and provides a wide range of utilities to debug, build, modify, or augment
//! the platform-integration of the application.
//!
//! See the documentation of the `osiris-platform` library for details on the
//! manifest, the supported target-platforms, the different supported
//! operations, as well as a general overview of the platform handling.
//!
//! This CLI is mainly a dispatcher of all the operations available in
//! `osiris_platform::op::*`. It is a simple clap-based CLI that forwards the
//! arguments to `osiris_platform` and visualizes the results.

use clap;
use osiris_platform;

struct Cli {
    cmd: clap::Command,
}

fn arg_platform_id(
    s: &str,
) -> Result<osiris_platform::platform::Id, clap::error::Error> {
    s.parse().map_err(
        |_| {
            clap::error::Error::raw(
                clap::error::ErrorKind::ValueValidation,
                "Invalid platform identifier",
            )
        }
    )
}

impl Cli {
    fn new() -> Self {
        let mut cmd;

        cmd = clap::Command::new("osiris-platform")
            .propagate_version(true)
            .subcommand_required(true)
            .about("Osiris Platform Tooling")
            .long_about("Manage the platform integration of rust applications")
            .version(clap::crate_version!());

        cmd = cmd.arg(
            clap::Arg::new("manifest")
                .long("manifest")
                .value_name("PATH")
                .help("Path to the platform manifest relative to the working directory")
                .default_value("./osiris-platform.toml")
                .value_parser(clap::builder::ValueParser::os_string())
        );

        cmd = cmd.subcommand(
            clap::Command::new("emerge")
                .about("Create a persisting platform integration")
                .arg(
                    clap::Arg::new("platform")
                        .long("platform")
                        .value_name("NAME")
                        .help("Name of the target platform to operate on")
                        .required(true)
                        .value_parser(arg_platform_id)
                )
                .arg(
                    clap::Arg::new("update")
                        .long("update")
                        .value_name("BOOL")
                        .help("Whether to allow updating existing platform integration")
                        .default_value("false")
                        .value_parser(clap::builder::ValueParser::bool())
                )
        );

        Self {
            cmd: cmd,
        }
    }

    fn manifest(
        &self,
        m: &clap::ArgMatches,
    ) -> Result<osiris_platform::manifest::Manifest, u8> {
        // Unwrap the manifest-path from the argument.
        let manifest_arg = m.get_raw("manifest");
        let mut manifest_iter = manifest_arg.expect("Cannot acquire manifest path");
        assert_eq!(manifest_iter.len(), 1);
        let manifest_path = manifest_iter.next().unwrap();

        // Parse the manifest from the path.
        let manifest = osiris_platform::manifest::Manifest::parse_path(
            std::path::Path::new(manifest_path)
        );
        match manifest {
            Err(_) => {
                eprintln!("Cannot parse platform manifest {:?}: ...", manifest_path);
                Err(1)
            },
            Ok(v) => {
                Ok(v)
            },
        }
    }

    fn op_emerge(
        &self,
        m: &clap::ArgMatches,
        m_op: &clap::ArgMatches,
    ) -> Result<(), u8> {
        let manifest = self.manifest(m)?;
        let platform = *m_op.get_one("platform").expect("Platform-flag lacks a value");
        let update = *m_op.get_one("update").expect("Update-flag lacks a value");

        match osiris_platform::op::emerge::emerge(
            &manifest,
            platform,
            None,
            update,
        ) {
            Err(osiris_platform::op::emerge::Error::Already) => {
                eprintln!("Cannot emerge platform integration: Platform code already present");
                Err(1)
            },
            Err(osiris_platform::op::emerge::Error::ManifestKey(key)) => {
                eprintln!("Cannot emerge platform integration: Manifest configuration missing '{}'", key);
                Err(1)
            },
            Err(osiris_platform::op::emerge::Error::DirectoryCreation(dir)) => {
                eprintln!("Cannot emerge platform integration: Failed to create directory {:?}", dir);
                Err(1)
            },
            Err(osiris_platform::op::emerge::Error::FileUpdate(file, error)) => {
                eprintln!("Cannot emerge platform integration: Failed to update {:?} ({})", file, error);
                Err(1)
            },
            Err(osiris_platform::op::emerge::Error::FileRemoval(file, error)) => {
                eprintln!("Cannot emerge platform integration: Failed to remove {:?} ({})", file, error);
                Err(1)
            },
            Ok(_) => {
                Ok(())
            },
        }
    }

    fn run(mut self) -> Result<(), u8> {
        let (m, r);

        r = self.cmd.try_get_matches_from_mut(
            std::env::args_os(),
        );

        match r {
            Ok(v) => m = v,
            Err(e) => {
                return match e.kind() {
                    clap::error::ErrorKind::DisplayHelp |
                    clap::error::ErrorKind::DisplayVersion => {
                        e.print().expect("Cannot write to STDERR");
                        Ok(())
                    },
                    clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand |
                    _ => {
                        e.print().expect("Cannot write to STDERR");
                        Err(2)
                    }
                }
            }
        }

        match m.subcommand() {
            Some(("emerge", m_op)) => self.op_emerge(&m, &m_op),
            _ => std::unreachable!(),
        }
    }
}

fn main() -> std::process::ExitCode {
    match Cli::new().run() {
        Ok(()) => 0.into(),
        Err(v) => v.into(),
    }
}
