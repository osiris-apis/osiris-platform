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
            clap::Command::new("build")
                .about("Build artifacts for the specified platform")
                .arg(
                    clap::Arg::new("platform")
                        .long("platform")
                        .value_name("NAME")
                        .help("ID of the target platform to operate on")
                        .required(true)
                        .value_parser(clap::builder::ValueParser::string())
                )
        );

        cmd = cmd.subcommand(
            clap::Command::new("emerge")
                .about("Create a persisting platform integration")
                .arg(
                    clap::Arg::new("platform")
                        .long("platform")
                        .value_name("NAME")
                        .help("ID of the target platform to operate on")
                        .required(true)
                        .value_parser(clap::builder::ValueParser::string())
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
    ) -> Result<
        (osiris_platform::manifest::Manifest, osiris_platform::manifest::ViewApplication),
        u8,
    > {
        // Unwrap the manifest-path from the argument.
        let manifest_arg = m.get_raw("manifest");
        let mut manifest_iter = manifest_arg.expect("Cannot acquire manifest path");
        assert_eq!(manifest_iter.len(), 1);
        let manifest_path = manifest_iter.next().unwrap();

        // Parse the manifest from the path.
        let manifest = osiris_platform::manifest::Manifest::parse_path(
            &std::path::Path::new(manifest_path)
        ).map_err(
            |_| {
                eprintln!("Cannot parse platform manifest {:?}: ...", manifest_path);
                1
            }
        )?;

        let view_application = manifest.raw.view_application().map_err(
            |v| {
                match v {
                    osiris_platform::manifest::ErrorView::MissingKey(v) => {
                        eprintln!("Cannot parse platform manifest: Missing configuration key '{}'", v);
                        1
                    },
                }
            }
        )?;

        Ok((manifest, view_application))
    }

    fn metadata(
        &self,
        path: &std::path::Path,
    ) -> Result<osiris_platform::cargo::Metadata, u8> {
        // Query cargo for its metadata via `cargo metadata`.
        match osiris_platform::cargo::Metadata::cargo(&path) {
            Err(osiris_platform::cargo::Error::Standalone) => {
                eprintln!("Cannot query cargo metadata: Not running as cargo sub-command");
                Err(1)
            },
            Err(osiris_platform::cargo::Error::Exec(error)) => {
                eprintln!("Cannot query cargo metadata: Execution of cargo could not commence ({})", error);
                Err(1)
            },
            Err(osiris_platform::cargo::Error::Cargo) => {
                eprintln!("Cannot query cargo metadata: Cargo failed executing");
                Err(1)
            },
            Err(osiris_platform::cargo::Error::Unicode(error)) => {
                eprintln!("Cannot query cargo metadata: Cargo returned invalid unicode data ({})", error);
                Err(1)
            },
            Err(osiris_platform::cargo::Error::Json) => {
                eprintln!("Cannot query cargo metadata: Cargo returned invalid JSON data");
                Err(1)
            },
            Err(osiris_platform::cargo::Error::Data) => {
                eprintln!("Cannot query cargo metadata: Cargo metadata lacks required fields");
                Err(1)
            },
            Ok(v) => {
                Ok(v)
            },
        }
    }

    fn platform<'manifest>(
        &self,
        m: &clap::ArgMatches,
        manifest: &'manifest osiris_platform::manifest::Manifest,
    ) -> Result<&'manifest osiris_platform::manifest::RawPlatform, u8> {
        let id: &String = m.get_one("platform").expect("Cannot acquire platform ID");

        match manifest.raw.platform_by_id(id) {
            Some(v) => Ok(v),
            None => {
                eprintln!("No platform with ID {}", id);
                Err(1)
            },
        }
    }

    fn op_build(
        &self,
        m: &clap::ArgMatches,
        m_op: &clap::ArgMatches,
    ) -> Result<(), u8> {
        let (manifest, view_application) = self.manifest(m)?;
        let metadata = self.metadata(&manifest.absolute_path(&view_application.path))?;
        let platform = self.platform(m_op, &manifest)?;

        match osiris_platform::op::build::build(
            &manifest,
            &metadata,
            platform,
        ) {
            Err(osiris_platform::op::build::Error::ManifestKey(key)) => {
                eprintln!("Cannot build platform integration: Manifest configuration missing '{}'", key);
                Err(1)
            },
            Err(osiris_platform::op::build::Error::PlatformDirectory(dir)) => {
                eprintln!("Cannot build platform integration: Failed to access platform directory {:?}", dir);
                Err(1)
            },
            Err(osiris_platform::op::build::Error::DirectoryCreation(dir)) => {
                eprintln!("Cannot build platform integration: Failed to create directory {:?}", dir);
                Err(1)
            },
            Err(osiris_platform::op::build::Error::FileUpdate(file, error)) => {
                eprintln!("Cannot build platform integration: Failed to update {:?} ({})", file, error);
                Err(1)
            },
            Err(osiris_platform::op::build::Error::FileRemoval(file, error)) => {
                eprintln!("Cannot build platform integration: Failed to remove {:?} ({})", file, error);
                Err(1)
            },
            Err(osiris_platform::op::build::Error::Exec(cmd, error)) => {
                eprintln!("Cannot build platform integration: Failed to invoke '{}' ({})", cmd, error);
                Err(1)
            },
            Err(osiris_platform::op::build::Error::Build) => {
                eprintln!("Cannot build platform integration: Platform build failed");
                Err(1)
            },
            Ok(_) => {
                Ok(())
            },
        }
    }

    fn op_emerge(
        &self,
        m: &clap::ArgMatches,
        m_op: &clap::ArgMatches,
    ) -> Result<(), u8> {
        let (manifest, _) = self.manifest(m)?;
        let platform = self.platform(m_op, &manifest)?;
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
            Err(osiris_platform::op::emerge::Error::PlatformDirectory(dir)) => {
                eprintln!("Cannot emerge platform integration: Failed to access platform directory {:?}", dir);
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
            Some(("build", m_op)) => self.op_build(&m, &m_op),
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
