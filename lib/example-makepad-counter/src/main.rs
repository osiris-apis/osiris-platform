//! UI Counter via Makepad
//!
//! Main application entrypoint for all targets that compile as binary. Simply
//! jump into the entrypoint in the library.

use example_makepad_counter as lib;

fn main() {
    lib::app_main()
}
