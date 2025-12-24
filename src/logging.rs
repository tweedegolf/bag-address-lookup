use std::time::Instant;

/// Print a message prefixed with elapsed time since `start`.
pub fn log_with_elapsed(start: Instant, message: &str) {
    let elapsed = start.elapsed().as_secs_f32();
    println!("[{elapsed:>8.2}s] {message}");
}
