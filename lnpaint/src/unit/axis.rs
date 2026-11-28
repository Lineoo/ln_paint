use glam::{IVec2, UVec2, Vec2};

#[derive(Default, Debug, Clone, Copy)]
pub enum Axis {
    Left,
    Down,
    #[default]
    Right,
    Up,
}

impl Axis {
    /// Rotate counter-clockwise. Use [`rotate_cw`](Axis::rotate_cw) for clockwise.
    pub fn rotate_ccw(self) -> Axis {
        match self {
            Axis::Left => Axis::Down,
            Axis::Down => Axis::Right,
            Axis::Right => Axis::Up,
            Axis::Up => Axis::Left,
        }
    }

    /// Rotate clockwise. Use [`rotate_ccw`](Axis::rotate_ccw) for counter-clockwise.
    pub fn rotate_cw(self) -> Axis {
        match self {
            Axis::Left => Axis::Up,
            Axis::Up => Axis::Right,
            Axis::Right => Axis::Down,
            Axis::Down => Axis::Left,
        }
    }

    pub fn flip(self) -> Axis {
        match self {
            Axis::Left => Axis::Right,
            Axis::Right => Axis::Left,
            Axis::Down => Axis::Up,
            Axis::Up => Axis::Down,
        }
    }

    pub fn perpendicular(self, rhs: Axis) -> bool {
        self.is_horizontal() ^ rhs.is_horizontal()
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, Axis::Left | Axis::Right)
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, Axis::Up | Axis::Down)
    }

    pub fn is_positive(self) -> bool {
        matches!(self, Axis::Up | Axis::Right)
    }

    pub fn is_negative(self) -> bool {
        matches!(self, Axis::Left | Axis::Down)
    }

    pub fn horizontal(self) -> i32 {
        match self {
            Axis::Left => -1,
            Axis::Down => 0,
            Axis::Right => 1,
            Axis::Up => 0,
        }
    }

    pub fn vertical(self) -> i32 {
        match self {
            Axis::Left => 0,
            Axis::Down => -1,
            Axis::Right => 0,
            Axis::Up => 1,
        }
    }

    pub fn positive(self) -> Axis {
        match self {
            Axis::Left | Axis::Right => Axis::Right,
            Axis::Down | Axis::Up => Axis::Up,
        }
    }

    pub fn negative(self) -> Axis {
        match self {
            Axis::Left | Axis::Right => Axis::Left,
            Axis::Down | Axis::Up => Axis::Down,
        }
    }

    /// -1 0 1
    pub fn direction(self) -> IVec2 {
        match self {
            Axis::Left => IVec2::NEG_X,
            Axis::Down => IVec2::NEG_Y,
            Axis::Right => IVec2::X,
            Axis::Up => IVec2::Y,
        }
    }

    /// 0.0 0.5 1.0
    pub fn anchor(self) -> Vec2 {
        self.direction().as_vec2() * 0.5 + 0.5
    }

    pub fn half(self, main: u32, cross: u32) -> UVec2 {
        match self.is_horizontal() {
            true => UVec2::new(main, cross),
            false => UVec2::new(cross, main),
        }
    }
}
