//! Persistent Platform Integration
//!
//! The `emerge` operation stores platform integration persistently on disk.
//! Unlike just-in-time integration at build time, this allows adjusting the
//! platform integration to specific needs and retaining modifications across
//! builds.

/// Emerge Errors
///
/// This is the exhaustive list of possible errors raised by the emerge
/// operation. See each error for details.
pub enum Error {
    /// Platform integration is already present and updating was not
    /// allowed by the caller.
    Already,
    /// Specified key required but missing in manifest.
    ManifestKey(String),
    /// Cannot access the specified platform directory.
    PlatformDirectory(std::ffi::OsString),
    /// Creation of the directory at the specified path failed.
    DirectoryCreation(std::ffi::OsString),
    /// Updating the file at the specified path failed with the given error.
    FileUpdate(std::ffi::OsString, std::io::Error),
    /// Removing the file at the specified path failed with the given error.
    FileRemoval(std::ffi::OsString, std::io::Error),
}

// Ensure directory exists
//
// Make sure the directory at the given path exists. Create the directory and
// its parent directories if necessary.
//
// This is a convenience helper around `std::fs::create_dir_all()`, but
// returning the local error `Error::DirectoryCreation` on failure.
fn ensure_dir(
    path: &std::path::Path,
) -> Result<(), Error> {
    std::fs::create_dir_all(path)
        .map_err(
            |_| Error::DirectoryCreation(path.as_os_str().to_os_string())
        )
}

// Update a file if required
//
// This writes the given content to the specified file, but only if the file
// content does not already match the new content. This avoids modifying a file
// unless necessary. Thus, the file timestamp is only modified if the content
// really changed.
//
// Note that this reads in the entire file content. Thus, use it only on
// trusted content.
fn update_file(
    path: &std::path::Path,
    content: &str,
) -> Result<(), Error> {
    // Open the file read+write and create it if it does not exist, yet.
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(
            |v| Error::FileUpdate(path.as_os_str().to_os_string(), v),
        )?;

    // Read the entire file content into memory.
    let mut old = String::new();
    <std::fs::File as std::io::Read>::read_to_string(&mut f, &mut old)
        .map_err(
            |v| Error::FileUpdate(path.as_os_str().to_os_string(), v),
        )?;

    // If the file has to be updated, rewind the position, truncate the file
    // and write the new contents.
    if old != content {
        <std::fs::File as std::io::Seek>::rewind(&mut f)
            .map_err(
                |v| Error::FileUpdate(path.as_os_str().to_os_string(), v),
            )?;

        f.set_len(0).map_err(
            |v| Error::FileUpdate(path.as_os_str().to_os_string(), v),
        )?;

        <std::fs::File as std::io::Write>::write_all(&mut f, content.as_bytes())
            .map_err(
                |v| Error::FileUpdate(path.as_os_str().to_os_string(), v),
            )?;
    }

    // Sync the file now to ensure errors are caught properly.
    f.sync_all().map_err(
        |v| Error::FileUpdate(path.as_os_str().to_os_string(), v),
    )?;

    Ok(())
}

// Unlink file if it exists
//
// Unlink the file at the specified path, but only if it exists. This is
// effectively like `std::fs::remove_file()`, but ignores errors about missing
// files.
fn unlink_file(path: &std::path::Path) -> Result<(), Error> {
    match std::fs::remove_file(path) {
        Err(v) if v.kind() != std::io::ErrorKind::NotFound => {
            Err(Error::FileRemoval(path.as_os_str().to_os_string(), v))
        },
        _ => {
            Ok(())
        }
    }
}

// Emerge Android `gradle.properties`
//
// `gradle.properties` is a key-value store read by Gradle before startup. It
// defines project-wide settings for Gradle. We uses it for:
//
//  * Enable `AndroidX`, the new Android middle-layer that allows using new
//    Android APIs on old devices.
//
//  * Set JVM parameters during the Gradle build (2G memory and UTF-8 files).
//
//  * Make `R` classes non-transitive to avoid pulling in resources from other
//    modules and adding build dependencies that usually needlessly slow down
//    the build.
fn emerge_android_gradle_properties(
    path: &mut std::path::PathBuf,
) -> Result<(), Error> {
    let content = concat!(
        "# Generated by osiris-platform\n",
        "org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n",
        "android.useAndroidX=true\n",
        "android.nonTransitiveRClass=true\n",
    );
    path.push("gradle.properties");
    update_file(path.as_path(), content)?;
    path.pop();
    Ok(())
}

// Emerge Android `local.properties`
//
// `local.properties` is a key-value store read by Gradle during startup,
// usually reserved for local project configuration that is not committed to
// the code base. We use it for:
//
//  * Define the path to the Android SDK.
fn emerge_android_local_properties(
    path: &mut std::path::PathBuf,
    sdk_path: Option<&String>,
) -> Result<(), Error> {
    path.push("local.properties");
    if let Some(v) = sdk_path {
        let content = format!(
            concat!(
                "# Generated by osiris-platform\n",
                "sdk.dir={0}\n",
            ),
            // Manifest verified: no newlines
            v,
        );
        update_file(path.as_path(), content.as_str())?;
    } else {
        unlink_file(path.as_path())?;
    }
    path.pop();
    Ok(())
}

// Emerge Android `settings.gradle`
//
// `settings.gradle` is the root configuration for Gradle. It is similar to
// `gradle.properties`, but is a full Gradle build file, rather than a
// key-value store. It is read before the `build.gradle` file. We use it to:
//
//  * Configure the Gradle module resolution behavior and specify which module
//    registries are used.
//
//  * Configure the root project name. This is used in file-names for build
//    artifacts.
fn emerge_android_settings_gradle(
    path: &mut std::path::PathBuf,
    appid: &str,
) -> Result<(), Error> {
    let content = format!(
        concat!(
            "// Generated by osiris-platform\n",
            "pluginManagement {{\n",
            "    repositories {{\n",
            "        google()\n",
            "        mavenCentral()\n",
            "        gradlePluginPortal()\n",
            "    }}\n",
            "}}\n",
            "dependencyResolutionManagement {{\n",
            "    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)\n",
            "    repositories {{\n",
            "        google()\n",
            "        mavenCentral()\n",
            "    }}\n",
            "}}\n",
            "rootProject.name = '{0}'\n",
        ),
        // Manifest verified: no quotes or backslashes
        appid,
    );
    path.push("settings.gradle");
    update_file(path.as_path(), content.as_str())?;
    path.pop();
    Ok(())
}

// Emerge Android `build.gradle`
//
// `build.gradle` is the root build file for Gradle. It defines the artifacts
// to build, using the Groovy configuration language.
fn emerge_android_build_gradle(
    path: &mut std::path::PathBuf,
    android_application_id: &str,
    namespace: &str,
    compile_sdk: u32,
    min_sdk: u32,
    target_sdk: u32,
    version_code: u32,
    version_name: &str,
) -> Result<(), Error> {
    let content = format!(
        concat!(
            "// Generated by osiris-platform\n",
            "plugins {{\n",
            "    id 'com.android.application' version '8.0.2'\n",
            "}}\n",
            "\n",
            "android {{\n",
            "    namespace '{1}'\n",
            "    compileSdk {2}\n",
            "\n",
            "    defaultConfig {{\n",
            "        applicationId '{0}'\n",
            "        minSdk {3}\n",
            "        targetSdk {4}\n",
            "        versionCode {5}\n",
            "        versionName '{6}'\n",
            "\n",
            "        testInstrumentationRunner 'androidx.test.runner.AndroidJUnitRunner'\n",
            "    }}\n",
            "\n",
            "    buildTypes {{\n",
            "        release {{\n",
            "            minifyEnabled false\n",
            "        }}\n",
            "    }}\n",
            "    compileOptions {{\n",
            "        sourceCompatibility JavaVersion.VERSION_1_8\n",
            "        targetCompatibility JavaVersion.VERSION_1_8\n",
            "    }}\n",
            "}}\n",
            "\n",
            "dependencies {{\n",
            "    implementation 'androidx.appcompat:appcompat:1.6.1'\n",
            "    implementation 'com.google.android.material:material:1.9.0'\n",
            "    implementation 'androidx.constraintlayout:constraintlayout:2.1.4'\n",
            "    testImplementation 'junit:junit:4.13.2'\n",
            "    androidTestImplementation 'androidx.test.ext:junit:1.1.5'\n",
            "    androidTestImplementation 'androidx.test.espresso:espresso-core:3.5.1'\n",
            "}}\n",
        ),
        // Manifest verified: no quotes or backslashes
        android_application_id,
        // Manifest verified: no quotes or backslashes
        namespace,
        compile_sdk,
        min_sdk,
        target_sdk,
        version_code,
        // Manifest verified: no quotes or backslashes
        version_name,
    );
    path.push("build.gradle");
    update_file(path.as_path(), content.as_str())?;
    path.pop();
    Ok(())
}

// Emerge Android `AndroidManifest.xml`
//
// Write the main application manifest according to the Android Application
// documentation. This manifest is the root application configuration and
// refers to all the embedded resources, including the entry-point activity.
fn emerge_android_manifest(
    path: &mut std::path::PathBuf,
    appid: &str,
) -> Result<(), Error> {
    let content = format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
            "<!-- Generated by osiris-platform -->\n",
            "<manifest\n",
            "    xmlns:android=\"http://schemas.android.com/apk/res/android\"\n",
            "    xmlns:tools=\"http://schemas.android.com/tools\">\n",
            "\n",
            "    <application\n",
            "        android:allowBackup=\"true\"\n",
            "        android:label=\"@string/app_name\"\n",
            "        android:supportsRtl=\"true\"\n",
            "        android:theme=\"@style/Theme.{0}\">\n",
            "        <activity\n",
            "            android:name=\".MainActivity\"\n",
            "            android:exported=\"true\">\n",
            "            <intent-filter>\n",
            "                <action android:name=\"android.intent.action.MAIN\" />\n",
            "                <category android:name=\"android.intent.category.LAUNCHER\" />\n",
            "            </intent-filter>\n",
            "        </activity>\n",
            "    </application>\n",
            "</manifest>\n",
        ),
        appid,
    );
    path.push("AndroidManifest.xml");
    update_file(path.as_path(), content.as_str())?;
    path.pop();
    Ok(())
}

// Emerge Android `activity_main.xml`
//
// This is the layout used by the main activity, defining the UI elements and
// their relations. This is referenced by `MainActivity` and used as default
// layout.
//
// This is a simple full-widget layout with a text-box showing "Hello World!".
fn emerge_android_activity_main(
    path: &mut std::path::PathBuf,
) -> Result<(), Error> {
    let content = format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
            "<!-- Generated by osiris-platform -->\n",
            "<androidx.constraintlayout.widget.ConstraintLayout\n",
            "    xmlns:android=\"http://schemas.android.com/apk/res/android\"\n",
            "    xmlns:app=\"http://schemas.android.com/apk/res-auto\"\n",
            "    xmlns:tools=\"http://schemas.android.com/tools\"\n",
            "    android:layout_width=\"match_parent\"\n",
            "    android:layout_height=\"match_parent\"\n",
            "    tools:context=\".MainActivity\">\n",
            "\n",
            "    <TextView\n",
            "        android:layout_width=\"wrap_content\"\n",
            "        android:layout_height=\"wrap_content\"\n",
            "        android:text=\"Hello World!\"\n",
            "        app:layout_constraintBottom_toBottomOf=\"parent\"\n",
            "        app:layout_constraintEnd_toEndOf=\"parent\"\n",
            "        app:layout_constraintStart_toStartOf=\"parent\"\n",
            "        app:layout_constraintTop_toTopOf=\"parent\" />\n",
            "</androidx.constraintlayout.widget.ConstraintLayout>\n",
        ),
    );
    path.push("activity_main.xml");
    update_file(path.as_path(), content.as_str())?;
    path.pop();
    Ok(())
}

// Emerge Android `strings.xml`
//
// The default string registry of the Android application is usually located in
// a `strings.xml`. This allows simple localization, as well as decoupling from
// the code-base.
fn emerge_android_strings(
    path: &mut std::path::PathBuf,
    name: &str,
) -> Result<(), Error> {
    let content = format!(
        concat!(
            "<!-- Generated by osiris-platform -->\n",
            "<resources>\n",
            "    <string name=\"app_name\">{0}</string>\n",
            "</resources>\n",
        ),
        name,
    );
    path.push("strings.xml");
    update_file(path.as_path(), content.as_str())?;
    path.pop();
    Ok(())
}

// Emerge Android `themes.xml`
//
// Define the base theme for the application. This is the theme referenced from
// the application manifest. No custom styles are added, just the default
// theme is inherited.
fn emerge_android_themes(
    path: &mut std::path::PathBuf,
    appid: &str,
) -> Result<(), Error> {
    let content = format!(
        concat!(
            "<!-- Generated by osiris-platform -->\n",
            "<resources xmlns:tools=\"http://schemas.android.com/tools\">\n",
            "    <style name=\"Theme.{0}\" parent=\"Theme.Material3.DayNight.NoActionBar\">\n",
            "    </style>\n",
            "</resources>\n",
        ),
        appid,
    );
    path.push("themes.xml");
    update_file(path.as_path(), content.as_str())?;
    path.pop();
    Ok(())
}

// Emerge Android `MainActivity.java`
//
// Write the main activity code, which is the entrypoint into the application.
// It sets `activity_main` as the content-view and recreates the base class
// from the saved state, if any.
fn emerge_android_main_activity(
    path: &mut std::path::PathBuf,
    namespace: &str,
) -> Result<(), Error> {
    let content = format!(
        concat!(
            "// Generated by osiris-platform\n",
            "package {0};\n",
            "\n",
            "import androidx.appcompat.app.AppCompatActivity;\n",
            "\n",
            "import android.os.Bundle;\n",
            "\n",
            "public class MainActivity extends AppCompatActivity {{\n",
            "    @Override\n",
            "    protected void onCreate(Bundle savedInstanceState) {{\n",
            "        super.onCreate(savedInstanceState);\n",
            "        setContentView(R.layout.activity_main);\n",
            "    }}\n",
            "}}\n",
        ),
        namespace,
    );
    path.push("MainActivity.java");
    update_file(path.as_path(), content.as_str())?;
    path.pop();
    Ok(())
}

// Android-specific backend to `emerge()`.
fn emerge_android(
    manifest: &crate::manifest::Manifest,
    _platform: &crate::manifest::RawPlatform,
    android: &crate::manifest::RawPlatformAndroid,
    mut path: std::path::PathBuf,
) -> Result<(), Error> {
    // Fetch manifest keys.

    let mut manifest_application_name = None;
    let mut manifest_application_id = None;
    let mut manifest_android_application_id = None;
    let mut manifest_android_namespace = None;
    let mut manifest_android_compile_sdk = None;
    let mut manifest_android_min_sdk = None;
    let mut manifest_android_target_sdk = None;
    let mut manifest_android_version_code = None;
    let mut manifest_android_version_name = None;
    let mut manifest_android_sdk_path = None;

    if let Some(application) = &manifest.raw.application {
        if let Some(v) = &application.name {
            manifest_application_name = Some(v);
        }
        if let Some(v) = &application.id {
            manifest_application_id = Some(v);
        }
    }

    if let Some(v) = &android.application_id {
        manifest_android_application_id = Some(v);
    }
    if let Some(v) = &android.namespace {
        manifest_android_namespace = Some(v);
    }
    if let Some(v) = android.compile_sdk {
        manifest_android_compile_sdk = Some(v);
    }
    if let Some(v) = android.min_sdk {
        manifest_android_min_sdk = Some(v);
    }
    if let Some(v) = android.target_sdk {
        manifest_android_target_sdk = Some(v);
    }
    if let Some(v) = android.version_code {
        manifest_android_version_code = Some(v);
    }
    if let Some(v) = &android.version_name {
        manifest_android_version_name = Some(v);
    }
    if let Some(v) = &android.sdk_path {
        manifest_android_sdk_path = Some(v);
    }

    // Unwrap required keys.

    let manifest_application_name = manifest_application_name.ok_or_else(
        || Error::ManifestKey("application.name".into()),
    )?;
    let manifest_application_id = manifest_application_id.ok_or_else(
        || Error::ManifestKey("application.id".into()),
    )?;
    let manifest_android_application_id = manifest_android_application_id.ok_or_else(
        || Error::ManifestKey("platform.android.application-id".into()),
    )?;
    let manifest_android_namespace = manifest_android_namespace.ok_or_else(
        || Error::ManifestKey("platform.android.namespace".into()),
    )?;
    let manifest_android_compile_sdk = manifest_android_compile_sdk.ok_or_else(
        || Error::ManifestKey("platform.android.compile-sdk".into()),
    )?;
    let manifest_android_min_sdk = manifest_android_min_sdk.ok_or_else(
        || Error::ManifestKey("platform.android.min-sdk".into()),
    )?;
    let manifest_android_target_sdk = manifest_android_target_sdk.ok_or_else(
        || Error::ManifestKey("platform.android.target-sdk".into()),
    )?;
    let manifest_android_version_code = manifest_android_version_code.ok_or_else(
        || Error::ManifestKey("platform.android.version-code".into()),
    )?;
    let manifest_android_version_name = manifest_android_version_name.ok_or_else(
        || Error::ManifestKey("platform.android.version-name".into()),
    )?;

    // Create the persistent files.

    emerge_android_gradle_properties(&mut path)?;
    emerge_android_local_properties(&mut path, manifest_android_sdk_path)?;
    emerge_android_settings_gradle(&mut path, &manifest_application_id)?;
    emerge_android_build_gradle(
        &mut path,
        &manifest_android_application_id,
        &manifest_android_namespace,
        manifest_android_compile_sdk,
        manifest_android_min_sdk,
        manifest_android_target_sdk,
        manifest_android_version_code,
        &manifest_android_version_name,
    )?;

    path.push("src");
    {
        ensure_dir(path.as_path())?;

        path.push("main");
        {
            ensure_dir(path.as_path())?;
            emerge_android_manifest(&mut path, &manifest_application_id)?;

            path.push("res");
            {
                ensure_dir(path.as_path())?;

                path.push("layout");
                {
                    ensure_dir(path.as_path())?;
                    emerge_android_activity_main(&mut path)?;
                }
                path.pop();

                path.push("values");
                {
                    ensure_dir(path.as_path())?;
                    emerge_android_strings(&mut path, &manifest_application_name)?;
                    emerge_android_themes(&mut path, &manifest_application_id)?;
                }
                path.pop();
            }
            path.pop();

            path.push("java");
            {
                // Create the java-style directory-tree based on the namespace.
                let mut ns_path = path.as_path().join(
                    manifest_android_namespace.replace(".", "/"),
                );
                ensure_dir(ns_path.as_path())?;
                emerge_android_main_activity(&mut ns_path, manifest_android_namespace)?;
            }
            path.pop();
        }
        path.pop();
    }
    path.pop();

    Ok(())
}

/// Emerge persistent platform integration
///
/// Write the platform integration for the specified platform to persistent
/// storage. The manifest is sourced for integration parameters. By default,
/// the integration is written to the platform directory for the given platform
/// as specified in the manifest. This base path can be overridden via the
/// `path_override` parameter.
///
/// This function will fail if the platform base directory for the specified
/// platform already exists, unless `update` is `true`. In this case old files
/// are updated to match the new platform integration, and old leftovers are
/// deleted.
pub fn emerge(
    manifest: &crate::manifest::Manifest,
    platform: &crate::manifest::RawPlatform,
    path_override: Option<&std::path::Path>,
    update: bool,
) -> Result<(), Error> {
    let v_platform_path;
    let mut path = std::path::PathBuf::new();

    // By default, we use the path specified in the manifest as platform
    // directory. However, an override can be provided by the caller. This
    // is useful to emerge into ephemeral build directories.
    let platform_path = if let Some(path_base) = path_override {
        path_base
    } else {
        v_platform_path = platform.path();
        v_platform_path.as_ref()
    };

    // Check for the platform path to exist and being accessible. If the path
    // points to something other than a directory, we fail with an error. If
    // the path points to an existing directory and updates are not allowed,
    // we fail. Otherwise, we create the path and continue.
    path.push(platform_path);
    match std::fs::metadata(&path) {
        Ok(v) => {
            if !v.is_dir() {
                return Err(Error::PlatformDirectory(path.as_os_str().to_os_string()));
            } else if !update {
                return Err(Error::Already);
            }
        },
        Err(v) => {
            if v.kind() != std::io::ErrorKind::NotFound {
                return Err(Error::PlatformDirectory(path.as_os_str().to_os_string()));
            }
            ensure_dir(path.as_path())?;
        },
    };

    // Invoke the platform-dependent handler. Grant the path-buf to it, so it
    // can reuse it for further operations.
    match platform.configuration {
        Some(crate::manifest::RawPlatformConfiguration::Android(ref v)) => {
            emerge_android(manifest, platform, v, path)
        },
        None => Ok(()),
    }
}
