//! Lightweight terminal-safe spinner frames for active states.

/// Return a spinner frame for the given animation tick.
pub fn spinner_frame(frame_count: u64) -> &'static str {
    const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
    FRAMES[(frame_count as usize) % FRAMES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_cycles_frames() {
        assert_eq!(spinner_frame(0), "⠋");
        assert_eq!(spinner_frame(1), "⠙");
        assert_eq!(spinner_frame(8), "⠋");
    }
}
