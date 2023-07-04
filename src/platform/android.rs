//! Android Platform Integration
//!
//! This module documents how to take a Rust application and turn it into an
//! application for the Android platform, what requirements are put on the Rust
//! application, and how to assemble the final artifacts for distribution. The
//! current Android platform does not have official recommendations for Rust
//! applications. However, the platform has guidelines for native code. Since
//! there are many possible ways to integrate Rust applications into Android,
//! this documentation follows the official Android developer recommendations
//! wherever possible. Alternatives may be discussed, but are ultimately not
//! pursued by this module.
//!
//! Applications on the Android platform are historically written in Java and
//! distributed as Java byte code[^dalvik]. Languages like Kotlin are also
//! supported, as well as compiled native code, but ultimately the application
//! entry-code must be Java byte code. This means, Rust applications must
//! either be compiled to Java byte code, or require a Java stub to load the
//! native Rust application. There are projects that can compile WASM or LLVM
//! byte code into Java byte code, and thus alleviate the need for Java stubs.
//! However, they are necessarily restricted by the capabilities and
//! requirements of the JVM. Hence, the official Android developer
//! recommendation is to use Java stub code that loads native libraries via
//! the Java Native Interface (JNI). The Android Native Development Kit (NDK)
//! provides a selection of pre-defined stubs that can be used instead of
//! shipping your own.
//!
//! The Android application build system uses Gradle with a custom Android
//! module. Native code can be pulled into this process via the Android
//! CMake integration and, for backwards compatibility, via the `ndk-build`
//! GNU-Make scripts. CMake integration is recommended. If desired, the
//! [corrosion-rs project](https://corrosion-rs.github.io) allows integrating
//! Cargo into CMake for automatic build dependency tracking. An alternative
//! is [rust-android-gradle](https://github.com/mozilla/rust-android-gradle)
//! by Mozilla to directly pull Cargo into Gradle. The latter does not
//! automatically benefit from optimizations the Android project puts into
//! the CMake integration for native code, though.
//!
//! The Osiris platform module places no restrictions on how to structure code
//! for an application. The Gradle based code and the Cargo based code can be
//! organized to your hearts content. The Osiris platform manifest encodes the
//! paths to the Gradle and Cargo project root directories, if required.
//! However, for applications targetting multiple platforms, the recommended
//! application layout is:
//!
//! ```text
//! <app>/
//! ├── Cargo.toml
//! ├── osiris-platform.toml
//! ├── platform/
//! │   ├── ...
//! │   └── android/
//! │       ├── build.gradle
//! │       ├── settings.gradle
//! │       └── app/
//! │           ├── build.gradle
//! │           └── src/
//! │               ├── main/
//! │               │   ├── AndroidManifest.xml
//! │               │   ├── java/
//! │               │   │   └── ...
//! │               │   └── res/
//! │               │       └── ...
//! │               └── native/
//! │                   ├── CMakeLists.txt
//! │                   └── ...
//! └── src/
//!     ├── lib.rs
//!     └── ...
//! ```
//!
//! [^dalvik]: Originally, Android used the Dalvik Virtual Machine, but later
//!            on replaced it with the Android Runtime (ART). For most
//!            purposes, they behave like the Java Virtual Machine (JVM).
