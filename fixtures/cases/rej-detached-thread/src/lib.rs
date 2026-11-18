//! Detached thread.

/// Fire and forget.
pub fn background() {
    drop(std::thread::spawn(|| {}));
}
