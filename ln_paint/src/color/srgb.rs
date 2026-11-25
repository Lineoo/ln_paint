use std::ops::Mul;

use glam::Vec4;

/// **`S(L)`** L = L_srgb
///
/// usually for painting
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Srgb(pub Vec4);

/// **`P(L)`** L = L_srgb
///
/// usually for UI
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SrgbLinear(pub Vec4);

/// **`L`** L = L_srgb
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SrgbLinearUnmultiplied(pub Vec4);

/// **`P(S(L))`** L = L_srgb
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SrgbPremultiplied(pub Vec4);

/// **`S(P(L))`** L = L_srgb
///
/// such a long name is intended
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SrgbLinearPremultipliedEncoded(pub Vec4);

impl Srgb {
    pub const BLACK: Srgb = Srgb(Vec4::W);
    pub const WHITE: Srgb = Srgb(Vec4::ONE);
    pub const TRANSPARENT: Srgb = Srgb(Vec4::ZERO);

    pub fn encode(v: f32) -> f32 {
        match v < 0.0031308 {
            true => v * 12.92,
            false => 1.055 * v.powf(1.0 / 2.4) - 0.055,
        }
    }

    pub fn decode(v: f32) -> f32 {
        match v <= 0.04045 {
            true => v / 12.92,
            false => ((v + 0.055) / 1.055).powf(2.4),
        }
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Srgb(self.0.with_w(a))
    }

    /// `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA`
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim().strip_prefix('#').unwrap_or(hex);
        assert!(
            matches!(hex.len(), 3 | 4 | 6 | 8),
            "invalid hex color: {hex}"
        );

        let digits: Vec<u8> = hex
            .bytes()
            .map(|b| (b as char).to_digit(16).expect("invalid hex digit") as u8)
            .collect();

        let channel = |high: u8, low: u8| (high * 16 + low) as f32 / 255.0;
        match digits.as_slice() {
            [r, g, b] => Srgb(Vec4::new(
                channel(*r, *r),
                channel(*g, *g),
                channel(*b, *b),
                1.0,
            )),
            [r, g, b, a] => Srgb(Vec4::new(
                channel(*r, *r),
                channel(*g, *g),
                channel(*b, *b),
                channel(*a, *a),
            )),
            [r0, r1, g0, g1, b0, b1] => Srgb(Vec4::new(
                channel(*r0, *r1),
                channel(*g0, *g1),
                channel(*b0, *b1),
                1.0,
            )),
            [r0, r1, g0, g1, b0, b1, a0, a1] => Srgb(Vec4::new(
                channel(*r0, *r1),
                channel(*g0, *g1),
                channel(*b0, *b1),
                channel(*a0, *a1),
            )),
            _ => unreachable!(),
        }
    }

    pub fn to_linear(self) -> SrgbLinear {
        self.to_linear_unmultiplied().to_multiplied()
    }

    pub fn to_linear_unmultiplied(self) -> SrgbLinearUnmultiplied {
        SrgbLinearUnmultiplied(Vec4::new(
            Srgb::decode(self.0.x),
            Srgb::decode(self.0.y),
            Srgb::decode(self.0.z),
            self.0.w,
        ))
    }

    pub fn mix(self, rhs: Srgb, factor: f32) -> Srgb {
        Srgb(self.0 * (1. - factor) + rhs.0 * factor)
    }
}

impl SrgbLinear {
    pub const BLACK: SrgbLinear = SrgbLinear(Vec4::W);
    pub const WHITE: SrgbLinear = SrgbLinear(Vec4::ONE);
    pub const TRANSPARENT: SrgbLinear = SrgbLinear(Vec4::ZERO);

    pub fn to_srgb(self) -> Srgb {
        self.to_unmultiplied().to_srgb()
    }

    pub fn to_unmultiplied(self) -> SrgbLinearUnmultiplied {
        SrgbLinearUnmultiplied(demul_alpha(self.0))
    }

    pub fn to_encoded(self) -> SrgbLinearPremultipliedEncoded {
        SrgbLinearPremultipliedEncoded(Vec4::new(
            Srgb::encode(self.0.x),
            Srgb::encode(self.0.y),
            Srgb::encode(self.0.z),
            self.0.w,
        ))
    }

    pub fn mix(self, rhs: SrgbLinear, factor: f32) -> SrgbLinear {
        SrgbLinear(self.0 * (1. - factor) + rhs.0 * factor)
    }
}

impl SrgbLinearUnmultiplied {
    pub fn to_multiplied(self) -> SrgbLinear {
        SrgbLinear(mul_alpha(self.0))
    }

    pub fn to_srgb(self) -> Srgb {
        Srgb(Vec4::new(
            Srgb::encode(self.0.x),
            Srgb::encode(self.0.y),
            Srgb::encode(self.0.z),
            self.0.w,
        ))
    }
}

impl SrgbLinearPremultipliedEncoded {
    pub fn to_rgba_bytes(self) -> [u8; 4] {
        [
            (self.0.x * 255.0).clamp(0.0, 255.0).round() as u8,
            (self.0.y * 255.0).clamp(0.0, 255.0).round() as u8,
            (self.0.z * 255.0).clamp(0.0, 255.0).round() as u8,
            (self.0.w * 255.0).clamp(0.0, 255.0).round() as u8,
        ]
    }
}

impl Mul<f32> for SrgbPremultiplied {
    type Output = SrgbPremultiplied;

    fn mul(self, rhs: f32) -> Self::Output {
        SrgbPremultiplied(self.0 * rhs)
    }
}

impl Mul<f32> for SrgbLinear {
    type Output = SrgbLinear;

    fn mul(self, rhs: f32) -> Self::Output {
        SrgbLinear(self.0 * rhs)
    }
}

/// x -> P(x)
pub fn mul_alpha(v: Vec4) -> Vec4 {
    return Vec4::new(v.x * v.w, v.y * v.w, v.z * v.w, v.w);
}

/// P(x) -> x
pub fn demul_alpha(v: Vec4) -> Vec4 {
    match v.w.is_normal() {
        true => Vec4::new(v.x / v.w, v.y / v.w, v.z / v.w, v.w),
        false => Vec4::ZERO,
    }
}
