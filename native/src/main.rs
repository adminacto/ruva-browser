#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
#[path = "linux_main.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "main_win.rs"]
mod platform;

fn main() {
    platform::main();
}
