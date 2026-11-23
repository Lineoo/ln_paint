use std::time::Duration;

use crate::input::pointer::PointerEvent;

/// How long a press must last to count as a hold.
const HOLD_TIME: Duration = Duration::from_millis(400);

/// Hold gesture, fed with pointer events.
///
/// Reports whether the current press has lasted `duration`, regardless of movement.
pub struct Hold {
    duration: Duration,
}

impl Hold {
    pub const fn new(duration: Duration) -> Self {
        Self { duration }
    }

    pub fn reached(&self, event: &PointerEvent) -> bool {
        match event {
            PointerEvent::Move(capture) => capture.start_instant.elapsed() >= self.duration,
            _ => false,
        }
    }
}

impl Default for Hold {
    fn default() -> Self {
        Self::new(HOLD_TIME)
    }
}
