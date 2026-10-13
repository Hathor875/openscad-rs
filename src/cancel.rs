//! Cooperative cancellation for long-running evaluations.
//!
//! The GUI registers the thread that runs an evaluation and can request a
//! cancel from the UI thread. Checks are thread-targeted so parallel
//! evaluations (and the test suite, which never registers) are unaffected.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::ThreadId;

static CANCELLED: AtomicBool = AtomicBool::new(false);
static TARGET: Mutex<Option<ThreadId>> = Mutex::new(None);

/// Mark the current thread as the one cancellable evaluation.
pub fn begin_cancellable_eval() {
    *TARGET.lock().unwrap() = Some(std::thread::current().id());
    CANCELLED.store(false, Ordering::Relaxed);
}

/// Drop any registration and clear the cancel flag (before spawning a run).
pub fn reset_evaluation() {
    CANCELLED.store(false, Ordering::Relaxed);
    *TARGET.lock().unwrap() = None;
}

/// Request cancellation of the registered evaluation thread.
pub fn cancel_evaluation() {
    CANCELLED.store(true, Ordering::Relaxed);
}

/// True when the current thread's evaluation has been cancelled.
pub fn evaluation_cancelled() -> bool {
    if !CANCELLED.load(Ordering::Relaxed) {
        return false;
    }
    match *TARGET.lock().unwrap() {
        Some(id) => id == std::thread::current().id(),
        None => false,
    }
}

/// The message returned by evaluation when cancelled.
pub const CANCELLED_MESSAGE: &str = "Evaluation cancelled";
