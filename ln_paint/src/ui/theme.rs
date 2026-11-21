use glam::{Vec2, Vec4};
use ln_world::{Element, Handle, World};

use crate::{
    color::{
        oklab::oklab_to_linear_srgb,
        srgb::{hex_to_srgb, srgb_gamma_decode},
    },
    ui::widget::button::ButtonTheme,
    unit::Axis,
    val::{Val, ValBind},
};

pub struct Theme {
    pub gray0: Vec4,
    pub gray1: Vec4,
    pub gray2: Vec4,
    pub gray3: Vec4,
    pub gray4: Vec4,
    pub theme: Vec4,
    pub distinct: Vec4,
    /// Brush preview band colors, in L.
    pub c0: Vec4,
    pub c1: Vec4,
    pub shadow_color: Vec4,
    pub shadow_offset: Vec2,
    pub shadow_blur: f32,
    pub roundness: f32,
    pub axis: Axis,
    pub anim_factor: f32,
}

pub struct ThemeVal {
    pub gray0: Handle<Val<Vec4>>,
    pub gray1: Handle<Val<Vec4>>,
    pub gray2: Handle<Val<Vec4>>,
    pub gray3: Handle<Val<Vec4>>,
    pub gray4: Handle<Val<Vec4>>,
    pub theme: Handle<Val<Vec4>>,
    pub distinct: Handle<Val<Vec4>>,
    pub c0: Handle<Val<Vec4>>,
    pub c1: Handle<Val<Vec4>>,
    pub shadow_color: Handle<Val<Vec4>>,
}

impl Theme {
    pub fn dark_mode() -> Self {
        Theme {
            gray0: srgb_gamma_decode(hex_to_srgb("#1f1f1f")),
            gray1: srgb_gamma_decode(hex_to_srgb("#3e3e3e")),
            gray2: srgb_gamma_decode(hex_to_srgb("#454545")),
            gray3: srgb_gamma_decode(hex_to_srgb("#676767")),
            gray4: srgb_gamma_decode(hex_to_srgb("#818181")),
            theme: srgb_gamma_decode(hex_to_srgb("#cc7353")),
            distinct: srgb_gamma_decode(hex_to_srgb("#fff")),
            c0: oklab_to_linear_srgb(Vec4::new(0.6, 0.08, 0.0, 1.0)),
            c1: oklab_to_linear_srgb(Vec4::new(0.5, -0.04, 0.0, 1.0)),
            shadow_color: srgb_gamma_decode(Vec4::new(0.0, 0.0, 0.0, 0.25)),
            shadow_offset: Vec2::new(0.0, -4.0),
            shadow_blur: 4.0,
            roundness: 4.0,
            axis: Axis::Right,
            anim_factor: 30.0,
        }
    }

    pub fn light_mode() -> Self {
        Theme {
            gray0: srgb_gamma_decode(hex_to_srgb("#ffffff")),
            gray1: srgb_gamma_decode(hex_to_srgb("#f2f2f2")),
            gray2: srgb_gamma_decode(hex_to_srgb("#e5e5e5")),
            gray3: srgb_gamma_decode(hex_to_srgb("#dbdbdb")),
            gray4: srgb_gamma_decode(hex_to_srgb("#b8b8b8")),
            theme: srgb_gamma_decode(hex_to_srgb("#dcba96")),
            distinct: srgb_gamma_decode(hex_to_srgb("#000000")),
            c0: oklab_to_linear_srgb(Vec4::new(0.8, 0.1, 0.0, 1.0)),
            c1: oklab_to_linear_srgb(Vec4::new(0.9, -0.06, 0.0, 1.0)),
            shadow_color: srgb_gamma_decode(Vec4::new(0.0, 0.0, 0.0, 0.25)),
            shadow_offset: Vec2::new(0.0, -4.0),
            shadow_blur: 4.0,
            roundness: 4.0,
            axis: Axis::Right,
            anim_factor: 30.0,
        }
    }
}

impl ThemeVal {
    pub fn bind(world: &World, theme: Handle<Theme>) -> ThemeVal {
        let bind = |f: fn(&Theme) -> Vec4| ValBind::fetch(world, theme, move |theme, _| f(theme));
        ThemeVal {
            gray0: bind(|t| t.gray0),
            gray1: bind(|t| t.gray1),
            gray2: bind(|t| t.gray2),
            gray3: bind(|t| t.gray3),
            gray4: bind(|t| t.gray4),
            theme: bind(|t| t.theme),
            distinct: bind(|t| t.distinct),
            c0: bind(|t| t.c0),
            c1: bind(|t| t.c1),
            shadow_color: bind(|t| t.shadow_color),
        }
    }

    pub fn button(&self) -> ButtonTheme {
        ButtonTheme {
            idle_color: self.gray1,
            hover_color: self.gray2,
            press_color: self.gray3,
            selected_color: self.gray3,
        }
    }

    pub fn tab_button(&self) -> ButtonTheme {
        ButtonTheme {
            idle_color: self.gray2,
            hover_color: self.gray3,
            press_color: self.gray0,
            selected_color: self.gray0,
        }
    }
}

impl Element for Theme {}
impl Element for ThemeVal {}
