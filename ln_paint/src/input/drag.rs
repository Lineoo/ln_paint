use glam::DVec2;

use crate::input::pointer::{PointerCapture, PointerEvent};

/// Screen-space distance from the press point that starts a drag.
const THRESHOLD: f64 = 0.005;

/// Drag gesture state, fed with pointer events.
///
/// `threshold` is compared against the screen-space distance from the press point.
pub struct Drag {
    threshold: f64,
    last: DVec2,
    active: bool,
}

pub struct DragEvent {
    pub capture: PointerCapture,
    pub status: DragStatus,
    /// screen-space movement since the previous update
    pub delta: DVec2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DragStatus {
    Start,
    Dragging,
    End,
}

impl Drag {
    pub const fn new(threshold: f64) -> Self {
        Self {
            threshold,
            last: DVec2::ZERO,
            active: false,
        }
    }

    pub fn update(&mut self, event: &PointerEvent) -> Option<DragEvent> {
        let capture = event.capture()?;
        let screen = capture.pointer.screen;

        let status = match event {
            PointerEvent::Press(_) => {
                self.last = screen;
                self.active = false;
                return None;
            }
            PointerEvent::Move(_) => match self.active {
                true => DragStatus::Dragging,
                false => {
                    let distance = (capture.pointer.screen - capture.start.screen).length();
                    if distance <= self.threshold {
                        return None;
                    }
                    self.active = true;
                    DragStatus::Start
                }
            },
            PointerEvent::Release(_) => {
                if !self.active {
                    return None;
                }
                self.active = false;
                DragStatus::End
            }
            _ => return None,
        };

        let delta = screen - self.last;
        self.last = screen;
        Some(DragEvent {
            capture: capture.clone(),
            status,
            delta,
        })
    }
}

impl Default for Drag {
    fn default() -> Self {
        Self::new(THRESHOLD)
    }
}
