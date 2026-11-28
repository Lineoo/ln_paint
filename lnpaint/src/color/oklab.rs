use glam::{Mat3, Vec2, Vec4};

use crate::color::srgb::{SrgbLinear, SrgbLinearUnmultiplied};

/// **`L_oklab`**
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Oklab(pub Vec4);

/// **`L_oklch`**
///
/// hue in radians.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Oklch(pub Vec4);

impl Oklab {
    pub fn clip_srgb(self) -> Self {
        let l = self.0.x.clamp(0.0, 1.0);
        let (a, b) = (self.0.y, self.0.z);
        let c = (a * a + b * b).sqrt();
        if !c.is_normal() {
            return self;
        }
        let t = find_gamut_intersection(a / c, b / c, l, c, l)
            .min(1.0)
            .max(0.0);
        Oklab(Vec4::new(l, t * a, t * b, self.0.w))
    }

    pub fn from_srgb_linear(srgb: SrgbLinear) -> Self {
        Oklab::from_srgb_linear_unmultiplied(srgb.to_unmultiplied())
    }

    pub fn from_srgb_linear_unmultiplied(srgb: SrgbLinearUnmultiplied) -> Self {
        let lms = RGB2LMS * srgb.0.truncate();
        let lms_ = lms.powf(1. / 3.);
        Oklab((LMS2OKLAB * lms_).extend(srgb.0.w))
    }

    pub fn to_srgb_linear(self) -> SrgbLinear {
        self.to_srgb_linear_unmultiplied().to_multiplied()
    }

    pub fn to_srgb_linear_unmultiplied(self) -> SrgbLinearUnmultiplied {
        let lms_ = OKLAB2LMS * self.0.truncate();
        let lms = lms_ * lms_ * lms_;
        SrgbLinearUnmultiplied((LMS2RGB * lms).extend(self.0.w))
    }

    pub fn to_oklch(self) -> Oklch {
        Oklch(Vec4::new(
            self.0.x,
            Vec2::new(self.0.y, self.0.z).length(),
            f32::atan2(self.0.z, self.0.y),
            self.0.w,
        ))
    }
}

impl Oklch {
    pub fn to_oklab(self) -> Oklab {
        Oklab(Vec4::new(
            self.0.x,
            self.0.y * self.0.z.cos(),
            self.0.y * self.0.z.sin(),
            self.0.w,
        ))
    }
}

const OKLAB2LMS: Mat3 = Mat3::from_cols_array_2d(&[
    [1.0000000000, 1.0000000000, 1.0000000000],
    [0.3963377774, -0.1055613458, -0.0894841775],
    [0.2158037573, -0.0638541728, -1.2914855480],
]);

const LMS2RGB: Mat3 = Mat3::from_cols_array_2d(&[
    [4.0767416621, -1.2684380046, -0.0041960863],
    [-3.3077115913, 2.6097574011, -0.7034186147],
    [0.2309699292, -0.3413193965, 1.7076147010],
]);

const RGB2LMS: Mat3 = Mat3::from_cols_array_2d(&[
    [0.4122214708, 0.2119034982, 0.0883024619],
    [0.5363325363, 0.6806995451, 0.2817188376],
    [0.0514459929, 0.1073969566, 0.6299787005],
]);

const LMS2OKLAB: Mat3 = Mat3::from_cols_array_2d(&[
    [0.2104542553, 1.9779984951, 0.0259040371],
    [0.7936177850, -2.4285922050, 0.7827717662],
    [-0.0040720468, 0.4505937099, -0.8086757660],
]);

/// L_oklab -> L_srgb
#[deprecated]
pub fn oklab_to_linear_srgb(oklab: Vec4) -> Vec4 {
    let lms_ = OKLAB2LMS * oklab.truncate();
    let lms = lms_ * lms_ * lms_;
    (LMS2RGB * lms).extend(oklab.w)
}

/// L_oklab -> L_srgb
#[deprecated]
pub fn linear_srgb_to_oklab(rgba: Vec4) -> Vec4 {
    let lms = RGB2LMS * rgba.truncate();
    let lms_ = lms.powf(1. / 3.);
    (LMS2OKLAB * lms_).extend(rgba.w)
}

/// L_oklab -> L_oklch
#[deprecated]
pub fn oklab_to_oklch(oklab: Vec4) -> Vec4 {
    Vec4::new(
        oklab.x,
        Vec2::new(oklab.y, oklab.z).length(),
        f32::atan2(oklab.z, oklab.y),
        oklab.w,
    )
}

/// L_oklch -> L_oklab
#[deprecated]
pub fn oklch_to_oklab(oklch: Vec4) -> Vec4 {
    Vec4::new(
        oklch.x,
        oklch.y * oklch.z.cos(),
        oklch.y * oklch.z.sin(),
        oklch.w,
    )
}

#[deprecated]
pub fn oklab_clip_to_srgb_gamut(oklab: Vec4) -> Vec4 {
    let l = oklab.x.clamp(0.0, 1.0);
    let (a, b) = (oklab.y, oklab.z);
    let c = (a * a + b * b).sqrt();
    if !c.is_normal() {
        return oklab;
    }
    let t = find_gamut_intersection(a / c, b / c, l, c, l)
        .min(1.0)
        .max(0.0);
    Vec4::new(l, t * a, t * b, oklab.w)
}

/// Clips an Oklch color into the sRGB gamut while keeping lightness and hue
/// constant, only compressing chroma.
///
/// Ported from Björn Ottosson's "sRGB gamut clipping".
/// https://bottosson.github.io/posts/gamutclipping/
pub fn find_gamut_intersection(a: f32, b: f32, l1: f32, c1: f32, l0: f32) -> f32 {
    let (l, c) = find_cusp(a, b);

    let mut t;
    if (l1 - l0) * c - (l - l0) * c1 <= 0.0 {
        t = c * l0 / (c1 * l + c * (l0 - l1));
    } else {
        t = c * (l0 - 1.0) / (c1 * (l - 1.0) + c * (l0 - l1));

        let d_l = l1 - l0;
        let d_c = c1;

        let k_l = 0.3963377774 * a + 0.2158037573 * b;
        let k_m = -0.1055613458 * a - 0.0638541728 * b;
        let k_s = -0.0894841775 * a - 1.2914855480 * b;

        let l_dt = d_l + d_c * k_l;
        let m_dt = d_l + d_c * k_m;
        let s_dt = d_l + d_c * k_s;

        let l_ = l0 * (1.0 - t) + t * l1 + t * c1 * k_l;
        let m_ = l0 * (1.0 - t) + t * l1 + t * c1 * k_m;
        let s_ = l0 * (1.0 - t) + t * l1 + t * c1 * k_s;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let ldt = 3.0 * l_dt * l_ * l_;
        let mdt = 3.0 * m_dt * m_ * m_;
        let sdt = 3.0 * s_dt * s_ * s_;

        let ldt2 = 6.0 * l_dt * l_dt * l_;
        let mdt2 = 6.0 * m_dt * m_dt * m_;
        let sdt2 = 6.0 * s_dt * s_dt * s_;

        let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s - 1.0;
        let r1 = 4.0767416621 * ldt - 3.3077115913 * mdt + 0.2309699292 * sdt;
        let r2 = 4.0767416621 * ldt2 - 3.3077115913 * mdt2 + 0.2309699292 * sdt2;

        let u_r = r1 / (r1 * r1 - 0.5 * r * r2);
        let mut t_r = -r * u_r;

        let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s - 1.0;
        let g1 = -1.2684380046 * ldt + 2.6097574011 * mdt - 0.3413193965 * sdt;
        let g2 = -1.2684380046 * ldt2 + 2.6097574011 * mdt2 - 0.3413193965 * sdt2;

        let u_g = g1 / (g1 * g1 - 0.5 * g * g2);
        let mut t_g = -g * u_g;

        let b_ = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s - 1.0;
        let b1 = -0.0041960863 * ldt - 0.7034186147 * mdt + 1.7076147010 * sdt;
        let b2 = -0.0041960863 * ldt2 - 0.7034186147 * mdt2 + 1.7076147010 * sdt2;

        let u_b = b1 / (b1 * b1 - 0.5 * b_ * b2);
        let mut t_b = -b_ * u_b;

        if u_r < 0.0 {
            t_r = f32::MAX;
        }
        if u_g < 0.0 {
            t_g = f32::MAX;
        }
        if u_b < 0.0 {
            t_b = f32::MAX;
        }

        t += t_r.min(t_g.min(t_b));
    }

    t
}

fn find_cusp(a: f32, b: f32) -> (f32, f32) {
    let s_cusp = compute_max_saturation(a, b);
    let SrgbLinear(rgba) = Oklab(Vec4::new(1.0, s_cusp * a, s_cusp * b, 1.0)).to_srgb_linear();
    let (r, g, b) = (rgba.x, rgba.y, rgba.z);
    let l_cusp = (1.0 / r.max(g).max(b)).cbrt();
    let c_cusp = l_cusp * s_cusp;
    (l_cusp, c_cusp)
}

fn compute_max_saturation(a: f32, b: f32) -> f32 {
    let (k0, k1, k2, k3, k4, wl, wm, ws);
    if -1.88170328 * a - 0.80936493 * b > 1.0 {
        (k0, k1, k2, k3, k4, wl, wm, ws) = (
            1.19086277,
            1.76576728,
            0.59662641,
            0.75515197,
            0.56771245,
            4.0767416621,
            -3.3077115913,
            0.2309699292,
        );
    } else if 1.81444104 * a - 1.19445276 * b > 1.0 {
        (k0, k1, k2, k3, k4, wl, wm, ws) = (
            0.73956515,
            -0.45954404,
            0.08285427,
            0.12541070,
            0.14503204,
            -1.2684380046,
            2.6097574011,
            -0.3413193965,
        );
    } else {
        (k0, k1, k2, k3, k4, wl, wm, ws) = (
            1.35733652,
            -0.00915799,
            -1.15130210,
            -0.50559606,
            0.00692167,
            -0.0041960863,
            -0.7034186147,
            1.7076147010,
        );
    }

    let s = k0 + k1 * a + k2 * b + k3 * a * a + k4 * a * b;

    let k_l = 0.3963377774 * a + 0.2158037573 * b;
    let k_m = -0.1055613458 * a - 0.0638541728 * b;
    let k_s = -0.0894841775 * a - 1.2914855480 * b;

    let l_ = 1.0 + s * k_l;
    let m_ = 1.0 + s * k_m;
    let s_ = 1.0 + s * k_s;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let l_ds = 3.0 * k_l * l_ * l_;
    let m_ds = 3.0 * k_m * m_ * m_;
    let s_ds = 3.0 * k_s * s_ * s_;

    let l_ds2 = 6.0 * k_l * k_l * l_;
    let m_ds2 = 6.0 * k_m * k_m * m_;
    let s_ds2 = 6.0 * k_s * k_s * s_;

    let f = wl * l + wm * m + ws * s3;
    let f1 = wl * l_ds + wm * m_ds + ws * s_ds;
    let f2 = wl * l_ds2 + wm * m_ds2 + ws * s_ds2;

    s - f * f1 / (f1 * f1 - 0.5 * f * f2)
}
