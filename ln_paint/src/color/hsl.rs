use glam::{Vec3, Vec4};

use crate::color::srgb::Srgb;

/// **`L_hsl`**
/// 
/// hue in [0.0, 1.0].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Hsl(pub Vec4);

impl Hsl {
    pub fn from_srgb(srgb: Srgb) -> Self {
        let rgba = srgb.0;
        let max = (rgba.x).max(rgba.y).max(rgba.z);
        let min = (rgba.x).min(rgba.y).min(rgba.z);

        let c = max - min;
        let l = (max + min) * 0.5;

        if c == 0.0 {
            return Hsl(Vec4::new(0.0, 0.0, l, rgba.w));
        }

        let s = c / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == rgba.x {
            ((rgba.y - rgba.z) / c).rem_euclid(6.0) / 6.0
        } else if max == rgba.y {
            ((rgba.z - rgba.x) / c + 2.0) / 6.0
        } else {
            ((rgba.x - rgba.y) / c + 4.0) / 6.0
        };

        Hsl(Vec4::new(h, s, l, rgba.w))
    }

    pub fn to_srgb(self) -> Srgb {
        let hsla = self.0;
        let color = ((((hsla.x * 6.0 + Vec3::new(0.0, 4.0, 2.0)) % 6.0) - 3.0).abs() - 1.0)
            .clamp(Vec3::splat(0.0), Vec3::splat(1.0));
        let srgb = hsla.z + hsla.y * (color - 0.5) * (1.0 - (2.0 * hsla.z - 1.0).abs());
        return Srgb(srgb.extend(hsla.w));
    }
}

/// L_hsl -> S(L_srgb)
#[deprecated]
pub fn hsl_to_srgb(hsla: Vec4) -> Vec4 {
    let rgb = ((((hsla.x * 6.0 + Vec3::new(0.0, 4.0, 2.0)) % 6.0) - 3.0).abs() - 1.0)
        .clamp(Vec3::splat(0.0), Vec3::splat(1.0));
    return (hsla.z + hsla.y * (rgb - 0.5) * (1.0 - (2.0 * hsla.z - 1.0).abs())).extend(hsla.w);
}

/// S(L_srgb) -> L_hsl
#[deprecated]
pub fn srgb_to_hsl(rgba: Vec4) -> Vec4 {
    let max = (rgba.x).max(rgba.y).max(rgba.z);
    let min = (rgba.x).min(rgba.y).min(rgba.z);

    let c = max - min;
    let l = (max + min) * 0.5;

    if c == 0.0 {
        return Vec4::new(0.0, 0.0, l, rgba.w);
    }

    let s = c / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == rgba.x {
        ((rgba.y - rgba.z) / c).rem_euclid(6.0) / 6.0
    } else if max == rgba.y {
        ((rgba.z - rgba.x) / c + 2.0) / 6.0
    } else {
        ((rgba.x - rgba.y) / c + 4.0) / 6.0
    };

    Vec4::new(h, s, l, rgba.w)
}
