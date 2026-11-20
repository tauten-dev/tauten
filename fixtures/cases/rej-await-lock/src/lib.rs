//! A lock guard held across an await point.

use std::sync::Mutex;

/// Bumps a counter around another future.
pub async fn bump(counter: &Mutex<u8>, other: impl Future<Output = ()>) {
    if let Ok(mut guard) = counter.lock() {
        *guard = guard.saturating_add(1);
        other.await;
        drop(guard);
    }
}
