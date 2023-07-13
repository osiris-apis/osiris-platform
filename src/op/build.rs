//! Build Platform Integration
//!
//! Run a full build of the platform integration. This assembles all
//! application artifacts ready for distribution.

/// Build Errors
///
/// This is the exhaustive list of possible errors raised by the build
/// operation. See each error for details.
pub enum Error {
    /// Specified key required but missing in manifest.
    ManifestKey(&'static str),
    /// Cannot access the specified platform directory.
    PlatformDirectory(std::ffi::OsString),
    /// Cannot create the specified build artifact directory.
    DirectoryCreation(std::ffi::OsString),
    /// Updating the file at the specified path failed with the given error.
    FileUpdate(std::ffi::OsString, std::io::Error),
    /// Removing the file at the specified path failed with the given error.
    FileRemoval(std::ffi::OsString, std::io::Error),
    /// Command execution could not commence.
    Exec(String, std::io::Error),
    /// Platform build tools failed.
    Build,
}

fn cmd_gradle_project_prop(
    cmd: &mut std::process::Command,
    key: &str,
    value: &dyn std::convert::AsRef<std::ffi::OsStr>,
) {
    let mut arg = std::ffi::OsString::new();

    cmd.arg("--project-prop");
    arg.push(key);
    arg.push("=");
    arg.push(value.as_ref());
    cmd.arg(arg);
}

// Android-specific backend to `build()`.
fn build_android(
    _manifest: &crate::manifest::Manifest,
    _platform: &crate::manifest::RawPlatform,
    _android: &crate::manifest::RawPlatformAndroid,
    path_platform: std::path::PathBuf,
    mut path_build: std::path::PathBuf,
) -> Result<(), Error> {
    // Invoke Gradle
    //
    // We simply invoke the gradle-build with the requested target. Since
    // Gradle makes output-directories part of project configuration, we
    // need to override it to ensure build artifacts do not pollute the
    // sources.
    //
    // Note that gradle might spawn background daemons to run the build.
    // This is quite unfortunate, but we really do not want to deviate
    // from the Gradle defaults too much. Hence, run this in containers to
    // avoid all the gradle peculiarities.

    let bin = "gradle".to_string();
    let mut cmd = std::process::Command::new(&bin);

    cmd.arg("build");

    cmd.arg("--no-scan");
    cmd.arg("--no-watch-fs");
    cmd.arg("--parallel");
    cmd.arg("--quiet");

    cmd.arg("--project-dir");
    cmd.arg(path_platform.as_path());

    path_build.push("gradle-cache");
    cmd.arg("--project-cache-dir");
    cmd.arg(path_build.as_path());
    path_build.pop();

    path_build.push("gradle-build");
    cmd_gradle_project_prop(&mut cmd, "buildDir", &path_build);
    path_build.pop();

    cmd.stderr(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());

    let output = cmd.output().map_err(|v| Error::Exec(bin, v))?;
    if !output.status.success() {
        return Err(Error::Build);
    }

    Ok(())
}

/// Build platform integration
///
/// Perform a full build of the platform integration of the specified platform.
/// If no persistent platform integration is located in the platform directory,
/// an ephemeral platform integration is created and built.
///
/// The target directory of the current crate is used to store any build
/// artifacts. Hence, you likely want to call this through `cargo <external>`
/// to ensure cargo integration is hooked up as expected.
pub fn build(
    manifest: &crate::manifest::Manifest,
    metadata: &crate::cargo::Metadata,
    platform: &crate::manifest::RawPlatform,
) -> Result<(), Error> {
    let mut path_platform = std::path::PathBuf::new();
    let mut path_build = std::path::PathBuf::new();

    // Check for `./platform/<id>/` to exist and being accessible. Use the
    // path as specified in the manifest.
    path_platform.push(platform.path());
    let accessible = match std::fs::metadata(&path_platform) {
        Err(v) => {
            if v.kind() == std::io::ErrorKind::NotFound {
                false
            } else {
                return Err(Error::PlatformDirectory(path_platform.as_os_str().to_os_string()));
            }
        }
        Ok(m) => {
            if m.is_dir() {
                true
            } else {
                return Err(Error::PlatformDirectory(path_platform.as_os_str().to_os_string()));
            }
        }
    };

    // If `./platform/<platform>/` does not exist, create it in the build-root
    // and emerge ephemeral platform integration into it. The directory is
    // created at `<target>/osiris/platform/<platform>/`.
    if !accessible {
        path_platform.clear();
        path_platform.push(&metadata.target_directory);
        path_platform.push("osiris");
        path_platform.push("platform");
        path_platform.push(&platform.id);

        std::fs::create_dir_all(path_platform.as_path()).map_err(
            |_| Error::DirectoryCreation(path_platform.as_os_str().to_os_string())
        )?;

        match crate::op::emerge::emerge(
            manifest,
            platform,
            Some(path_platform.as_path()),
            true,
        ) {
            Err(crate::op::emerge::Error::Already) => {
                unreachable!("Emerging with updates allowed must not yield");
            },
            Err(crate::op::emerge::Error::ManifestKey(key)) => {
                return Err(Error::ManifestKey(key));
            },
            Err(crate::op::emerge::Error::PlatformDirectory(dir)) => {
                return Err(Error::PlatformDirectory(dir));
            },
            Err(crate::op::emerge::Error::DirectoryCreation(dir)) => {
                return Err(Error::DirectoryCreation(dir));
            },
            Err(crate::op::emerge::Error::FileUpdate(file, error)) => {
                return Err(Error::FileUpdate(file, error));
            },
            Err(crate::op::emerge::Error::FileRemoval(file, error)) => {
                return Err(Error::FileRemoval(file, error));
            },
            Ok(_) => {
            },
        }
    }

    // Create a build directory for all output artifacts of the build process.
    // Re-use the existing directory, if possible, to speed up builds. The
    // directory is created at: `<target>/osiris/build/<platform>`.
    path_build.push(&metadata.target_directory);
    path_build.push("osiris");
    path_build.push("build");
    path_build.push(&platform.id);
    std::fs::create_dir_all(path_build.as_path()).map_err(
        |_| Error::DirectoryCreation(path_build.as_os_str().to_os_string())
    )?;

    // Invoke the platform-dependent handler. Grant the path-buffers to it, so
    // it can reuse it for further operations.
    match platform.configuration {
        Some(crate::manifest::RawPlatformConfiguration::Android(ref v)) => {
            build_android(manifest, platform, v, path_platform, path_build)
        },
        None => Ok(()),
    }
}
