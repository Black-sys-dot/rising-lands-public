#[cfg(windows)]
fn main() {
    if let Err(message) = windows_client::platform::window::run() {
        eprintln!("Rising Lands: {message}");
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Rising Lands: the native client builds for Windows only.");
}
