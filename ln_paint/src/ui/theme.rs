use glam::Vec4;
use ln_world::{Element, Handle, World};

use crate::{
    color::{oklab::Oklch, srgb::SrgbLinear},
    val::{Val, ValBind},
};

pub struct Theme {
    pub color: ThemeColor,
    pub state: ThemeState,
    pub shape: ThemeShape,
    pub hue: Handle<Val<f32>>,
    pub scheme: Handle<Val<ThemeScheme>>,
    pub animation: f32,
}

#[rustfmt::skip]
pub struct ThemeColor {
    pub primary:                    Handle<Val<SrgbLinear>>,
    pub on_primary:                 Handle<Val<SrgbLinear>>,
    pub primary_container:          Handle<Val<SrgbLinear>>,
    pub on_primary_container:       Handle<Val<SrgbLinear>>,
    pub secondary:                  Handle<Val<SrgbLinear>>,
    pub on_secondary:               Handle<Val<SrgbLinear>>,
    pub secondary_container:        Handle<Val<SrgbLinear>>,
    pub on_secondary_container:     Handle<Val<SrgbLinear>>,
    pub tertiary:                   Handle<Val<SrgbLinear>>,
    pub on_tertiary:                Handle<Val<SrgbLinear>>,
    pub tertiary_container:         Handle<Val<SrgbLinear>>,
    pub on_tertiary_container:      Handle<Val<SrgbLinear>>,
    pub surface:                    Handle<Val<SrgbLinear>>,
    pub on_surface:                 Handle<Val<SrgbLinear>>,
    pub surface_variant:            Handle<Val<SrgbLinear>>,
    pub on_surface_variant:         Handle<Val<SrgbLinear>>,
    pub surface_container_highest:  Handle<Val<SrgbLinear>>,
    pub surface_container_high:     Handle<Val<SrgbLinear>>,
    pub surface_container:          Handle<Val<SrgbLinear>>,
    pub surface_container_low:      Handle<Val<SrgbLinear>>,
    pub surface_container_lowest:   Handle<Val<SrgbLinear>>,
    pub inverse_surface:            Handle<Val<SrgbLinear>>,
    pub inverse_on_surface:         Handle<Val<SrgbLinear>>,
    pub outline:                    Handle<Val<SrgbLinear>>,
    pub outline_variant:            Handle<Val<SrgbLinear>>,
    pub scrim:                      Handle<Val<SrgbLinear>>,
    pub shadow:                     Handle<Val<SrgbLinear>>,
}

pub struct ThemeState {
    pub hover: Handle<Val<f32>>,
    pub focus: Handle<Val<f32>>,
    pub press: Handle<Val<f32>>,
    pub drag: Handle<Val<f32>>,
}

pub struct ThemeShape {
    pub shadow_width: Handle<Val<f32>>,
    pub corner_radius: Handle<Val<f32>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ThemeScheme {
    Dark,
    DarkNeutral,
    Light,
    LightNeutral,
    Neutral,
    NeutralVariant,
}

#[rustfmt::skip]
mod consts {
    use std::f32::consts::TAU;

    // Chroma (standard, neutral)
    pub const PRIMARY           : (f32, f32) = (0.131, 0.022);
    pub const SECONDARY         : (f32, f32) = (0.044, 0.011);
    pub const TERTIARY          : (f32, f32) = (0.065, 0.022);
    pub const NEUTRAL_VARIANT   : (f32, f32) = (0.022, 0.011);
    pub const NEUTRAL           : (f32, f32) = (0.011, 0.000);

    // Hue
    pub const TERTIARY_HUE      : f32 = TAU / 6.;

    // Opacity
    pub const HOVER             : f32 = 0.08;
    pub const FOCUS             : f32 = 0.10;
    pub const PRESS             : f32 = 0.10;
    pub const DRAG              : f32 = 0.16;

    // Tone (Light, Dark, Neutral)
    pub const COLOR                     : (f32, f32, f32) = (0.40, 0.80, 0.80);
    pub const ON_COLOR                  : (f32, f32, f32) = (1.00, 0.20, 0.20);
    pub const COLOR_CONTAINER           : (f32, f32, f32) = (0.90, 0.30, 0.50);
    pub const ON_COLOR_CONTAINER        : (f32, f32, f32) = (0.30, 0.90, 1.00);
    pub const SURFACE                   : (f32, f32, f32) = (0.98, 0.06, 0.30);
    pub const ON_SURFACE                : (f32, f32, f32) = (0.10, 0.90, 1.00);
    pub const SURFACE_VARIANT           : (f32, f32, f32) = (0.90, 0.30, 0.55);
    pub const ON_SURFACE_VARIANT        : (f32, f32, f32) = (0.30, 0.80, 1.00);
    pub const SURFACE_CONTAINER_HIGHEST : (f32, f32, f32) = (0.90, 0.22, 0.32);
    pub const SURFACE_CONTAINER_HIGH    : (f32, f32, f32) = (0.92, 0.17, 0.36);
    pub const SURFACE_CONTAINER         : (f32, f32, f32) = (0.94, 0.12, 0.40);
    pub const SURFACE_CONTAINER_LOW     : (f32, f32, f32) = (0.96, 0.10, 0.44);
    pub const SURFACE_CONTAINER_LOWEST  : (f32, f32, f32) = (1.00, 0.04, 0.48);
    pub const INVERSE_SURFACE           : (f32, f32, f32) = (0.20, 0.90, 0.90);
    pub const INVERSE_ON_SURFACE        : (f32, f32, f32) = (0.95, 0.20, 0.20);
    pub const OUTLINE                   : (f32, f32, f32) = (0.50, 0.60, 0.60);
    pub const OUTLINE_VARIANT           : (f32, f32, f32) = (0.80, 0.30, 0.30);
    pub const SCRIM                     : (f32, f32, f32) = (0.00, 0.00, 0.00);
    pub const SHADOW                    : (f32, f32, f32) = (0.00, 0.00, 0.00);
}

impl Theme {
    pub fn from_world(world: &World) -> Self {
        Theme::dark(world)
    }

    pub fn light(world: &World) -> Self {
        Theme::source(
            world,
            world.insert(Val::new(0.67)),
            world.insert(Val::new(ThemeScheme::Light)),
        )
    }

    pub fn dark(world: &World) -> Self {
        Theme::source(
            world,
            world.insert(Val::new(0.67)),
            world.insert(Val::new(ThemeScheme::Dark)),
        )
    }

    pub fn source(world: &World, hue: Handle<Val<f32>>, scheme: Handle<Val<ThemeScheme>>) -> Self {
        Theme {
            color: ThemeColor::source(world, hue, scheme),
            state: ThemeState::from_world(world),
            shape: ThemeShape::from_world(world),
            hue,
            scheme,
            animation: 30.0,
        }
    }
}

impl ThemeColor {
    pub fn source(world: &World, hue: Handle<Val<f32>>, scheme: Handle<Val<ThemeScheme>>) -> Self {
        use consts::*;

        let bind = |(l_light, l_dark, l_neutral), (c_standard, c_neutral), h| {
            ValBind::new(world, move |world| {
                use ThemeScheme::*;

                let hue = world.fetch(hue).val;
                let scheme = world.fetch(scheme).val;
                // tweak OkLab's L value to get it closer to CIEDE2000
                let l = match scheme {
                    Dark | DarkNeutral => f32::powf(l_dark, 0.62),
                    Light | LightNeutral => f32::powf(l_light, 0.96),
                    Neutral | NeutralVariant => l_neutral,
                };
                let c = match scheme {
                    Dark | Light | NeutralVariant => c_standard,
                    DarkNeutral | LightNeutral | Neutral => c_neutral,
                };
                let oklch = Oklch(Vec4::new(l, c, hue + h, 1.0));
                oklch.to_oklab().to_srgb_linear()
            })
            .listen(hue)
            .listen(scheme)
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
            shadow_width: world.insert(Val::new(8.0)),
            corner_radius: world.insert(Val::new(4.0)),
        }
    }
}

impl Element for Theme {}
