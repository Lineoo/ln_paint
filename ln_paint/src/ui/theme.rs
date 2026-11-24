use glam::{Vec2, Vec4};
use ln_world::{Element, Handle, World};

use crate::{
    color::{
        oklab::{oklab_clip_to_srgb_gamut, oklab_to_linear_srgb, oklch_to_oklab},
        srgb::{hex_to_srgb, srgb_gamma_decode},
    },
    ui::widget::button::ButtonTheme,
    unit::Axis,
    val::{Val, ValBind},
};

pub struct Theme {
    pub color: ThemeColor,
    pub state: ThemeState,
    pub shape: ThemeShape,
}

#[rustfmt::skip]
pub struct ThemeColor {
    pub primary:                    Handle<Val<Vec4>>,
    pub on_primary:                 Handle<Val<Vec4>>,
    pub primary_container:          Handle<Val<Vec4>>,
    pub on_primary_container:       Handle<Val<Vec4>>,
    pub secondary:                  Handle<Val<Vec4>>,
    pub on_secondary:               Handle<Val<Vec4>>,
    pub secondary_container:        Handle<Val<Vec4>>,
    pub on_secondary_container:     Handle<Val<Vec4>>,
    pub tertiary:                   Handle<Val<Vec4>>,
    pub on_tertiary:                Handle<Val<Vec4>>,
    pub tertiary_container:         Handle<Val<Vec4>>,
    pub on_tertiary_container:      Handle<Val<Vec4>>,
    pub surface:                    Handle<Val<Vec4>>,
    pub on_surface:                 Handle<Val<Vec4>>,
    pub surface_variant:            Handle<Val<Vec4>>,
    pub on_surface_variant:         Handle<Val<Vec4>>,
    pub surface_container_highest:  Handle<Val<Vec4>>,
    pub surface_container_high:     Handle<Val<Vec4>>,
    pub surface_container:          Handle<Val<Vec4>>,
    pub surface_container_low:      Handle<Val<Vec4>>,
    pub surface_container_lowest:   Handle<Val<Vec4>>,
    pub inverse_surface:            Handle<Val<Vec4>>,
    pub inverse_on_surface:         Handle<Val<Vec4>>,
    pub outline:                    Handle<Val<Vec4>>,
    pub outline_variant:            Handle<Val<Vec4>>,
    pub scrim:                      Handle<Val<Vec4>>,
    pub shadow:                     Handle<Val<Vec4>>,
}

pub struct ThemeState {
    pub hover: Handle<Val<f32>>,
    pub focus: Handle<Val<f32>>,
    pub press: Handle<Val<f32>>,
    pub drag: Handle<Val<f32>>,
}

pub struct ThemeShape {
    pub shadow_width: Handle<Val<f32>>,
    pub roundness: Handle<Val<f32>>,
}

#[rustfmt::skip]
mod consts {
    use std::f32::consts::TAU;

    // Chroma
    pub const PRIMARY           : f32 = 0.131;
    pub const SECONDARY         : f32 = 0.044;
    pub const TERTIARY          : f32 = 0.065;
    pub const NEUTRAL_VARIANT   : f32 = 0.022;
    pub const NEUTRAL           : f32 = 0.011;

    // Hue
    pub const TERTIARY_HUE      : f32 = TAU / 6.;

    // State Layer Opacity
    pub const HOVER             : f32 = 0.08;
    pub const FOCUS             : f32 = 0.10;
    pub const PRESS             : f32 = 0.10;
    pub const DRAG              : f32 = 0.16;

    // Lightness (Tone)
    pub const COLOR                     : (f32, f32) = (0.40, 0.80);
    pub const ON_COLOR                  : (f32, f32) = (1.00, 0.20);
    pub const COLOR_CONTAINER           : (f32, f32) = (0.90, 0.30);
    pub const ON_COLOR_CONTAINER        : (f32, f32) = (0.30, 0.90);
    pub const SURFACE                   : (f32, f32) = (0.98, 0.06);
    pub const ON_SURFACE                : (f32, f32) = (0.10, 0.90);
    pub const SURFACE_VARIANT           : (f32, f32) = (0.90, 0.30);
    pub const ON_SURFACE_VARIANT        : (f32, f32) = (0.30, 0.80);
    pub const SURFACE_CONTAINER_HIGHEST : (f32, f32) = (0.90, 0.22);
    pub const SURFACE_CONTAINER_HIGH    : (f32, f32) = (0.92, 0.17);
    pub const SURFACE_CONTAINER         : (f32, f32) = (0.94, 0.12);
    pub const SURFACE_CONTAINER_LOW     : (f32, f32) = (0.96, 0.10);
    pub const SURFACE_CONTAINER_LOWEST  : (f32, f32) = (1.00, 0.04);
    pub const INVERSE_SURFACE           : (f32, f32) = (0.20, 0.90);
    pub const INVERSE_ON_SURFACE        : (f32, f32) = (0.95, 0.20);
    pub const OUTLINE                   : (f32, f32) = (0.50, 0.60);
    pub const OUTLINE_VARIANT           : (f32, f32) = (0.80, 0.30);
    pub const SCRIM                     : (f32, f32) = (0.00, 0.00);
    pub const SHADOW                    : (f32, f32) = (0.00, 0.00);
}

impl Theme {
    pub fn from_world(world: &World) -> Self {
        Theme {
            color: ThemeColor::from_world(world),
            state: ThemeState::from_world(world),
            shape: ThemeShape::from_world(world),
        }
    }
}

impl ThemeColor {
    pub fn from_world(world: &World) -> Self {
        ThemeColor::source(
            world,
            world.insert(Val::new(0.67)),
            world.insert(Val::new(true)),
        )
    }

    pub fn source(world: &World, hue: Handle<Val<f32>>, dark: Handle<Val<bool>>) -> Self {
        use consts::*;

        let bind = |(l_light, l_dark), c, h| {
            ValBind::new(world, move |world| {
                let hue = world.fetch(hue).val;
                let dark = world.fetch(dark).val;
                let mut oklch = Vec4::ONE;
                oklch.x = match dark {
                    true => l_light,
                    false => l_dark,
                };
                oklch.y = c;
                oklch.z = hue + h;
                oklab_to_linear_srgb(oklab_clip_to_srgb_gamut(oklch_to_oklab(oklch)))
            })
            .listen(hue)
            .listen(dark)
            .finish()
        };

        #[cfg_attr(rustfmt, rustfmt::skip)]
        ThemeColor {
            primary:                    bind(COLOR,                     PRIMARY,            0.0         ),
            on_primary:                 bind(ON_COLOR,                  PRIMARY,            0.0         ),
            primary_container:          bind(COLOR_CONTAINER,           PRIMARY,            0.0         ),
            on_primary_container:       bind(ON_COLOR_CONTAINER,        PRIMARY,            0.0         ),
            secondary:                  bind(COLOR,                     SECONDARY,          0.0         ),
            on_secondary:               bind(ON_COLOR,                  SECONDARY,          0.0         ),
            secondary_container:        bind(COLOR_CONTAINER,           SECONDARY,          0.0         ),
            on_secondary_container:     bind(ON_COLOR_CONTAINER,        SECONDARY,          0.0         ),
            tertiary:                   bind(COLOR,                     TERTIARY,           TERTIARY_HUE),
            on_tertiary:                bind(ON_COLOR,                  TERTIARY,           TERTIARY_HUE),
            tertiary_container:         bind(COLOR_CONTAINER,           TERTIARY,           TERTIARY_HUE),
            on_tertiary_container:      bind(ON_COLOR_CONTAINER,        TERTIARY,           TERTIARY_HUE),
            surface:                    bind(SURFACE,                   NEUTRAL,            0.0         ),
            on_surface:                 bind(ON_SURFACE,                NEUTRAL,            0.0         ),
            surface_variant:            bind(SURFACE_VARIANT,           NEUTRAL_VARIANT,    0.0         ),
            on_surface_variant:         bind(ON_SURFACE_VARIANT,        NEUTRAL_VARIANT,    0.0         ),
            surface_container_highest:  bind(SURFACE_CONTAINER_HIGHEST, NEUTRAL,            0.0         ),
            surface_container_high:     bind(SURFACE_CONTAINER_HIGH,    NEUTRAL,            0.0         ),
            surface_container:          bind(SURFACE_CONTAINER,         NEUTRAL,            0.0         ),
            surface_container_low:      bind(SURFACE_CONTAINER_LOW,     NEUTRAL,            0.0         ),
            surface_container_lowest:   bind(SURFACE_CONTAINER_LOWEST,  NEUTRAL,            0.0         ),
            inverse_surface:            bind(INVERSE_SURFACE,           NEUTRAL,            0.0         ),
            inverse_on_surface:         bind(INVERSE_ON_SURFACE,        NEUTRAL,            0.0         ),
            outline:                    bind(OUTLINE,                   NEUTRAL_VARIANT,    0.0         ),
            outline_variant:            bind(OUTLINE_VARIANT,           NEUTRAL_VARIANT,    0.0         ),
            scrim:                      bind(SCRIM,                     NEUTRAL,            0.0         ),
            shadow:                     bind(SHADOW,                    NEUTRAL,            0.0         ),
        }
    }
}

impl ThemeState {
    pub fn from_world(world: &World) -> Self {
        use consts::*;

        ThemeState {
            hover: world.insert(Val::new(HOVER)),
            focus: world.insert(Val::new(FOCUS)),
            press: world.insert(Val::new(PRESS)),
            drag: world.insert(Val::new(DRAG)),
        }
    }
}

impl ThemeShape {
    pub fn from_world(world: &World) -> Self {
        ThemeShape {
            shadow_width: world.insert(Val::new(4.0)),
            roundness: world.insert(Val::new(4.0)),
        }
    }
}

pub struct LegacyTheme {
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

pub struct LegacyThemeVal {
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

impl LegacyTheme {
    pub fn dark_mode() -> Self {
        LegacyTheme {
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
        LegacyTheme {
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

impl LegacyThemeVal {
    pub fn bind(world: &World, theme: Handle<LegacyTheme>) -> LegacyThemeVal {
        let bind =
            |f: fn(&LegacyTheme) -> Vec4| ValBind::fetch(world, theme, move |theme, _| f(theme));
        LegacyThemeVal {
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

impl Element for LegacyTheme {}
impl Element for LegacyThemeVal {}
