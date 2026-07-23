#[cfg(target_os = "linux")]
#[path = "linux_main.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "main_win_chromium.rs"]
mod platform;

fn main() {
    platform::main();
}
