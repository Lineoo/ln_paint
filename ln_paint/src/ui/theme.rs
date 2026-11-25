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
    pub dark: Handle<Val<bool>>,
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

    // Opacity
    pub const HOVER             : f32 = 0.08;
    pub const FOCUS             : f32 = 0.10;
    pub const PRESS             : f32 = 0.10;
    pub const DRAG              : f32 = 0.16;

    // Tone
    pub const COLOR                     : (u32, u32) = ( 40, 80);
    pub const ON_COLOR                  : (u32, u32) = (100, 20);
    pub const COLOR_CONTAINER           : (u32, u32) = ( 90, 30);
    pub const ON_COLOR_CONTAINER        : (u32, u32) = ( 30, 90);
    pub const SURFACE                   : (u32, u32) = ( 98,  6);
    pub const ON_SURFACE                : (u32, u32) = ( 10, 90);
    pub const SURFACE_VARIANT           : (u32, u32) = ( 90, 30);
    pub const ON_SURFACE_VARIANT        : (u32, u32) = ( 30, 80);
    pub const SURFACE_CONTAINER_HIGHEST : (u32, u32) = ( 90, 22);
    pub const SURFACE_CONTAINER_HIGH    : (u32, u32) = ( 92, 17);
    pub const SURFACE_CONTAINER         : (u32, u32) = ( 94, 12);
    pub const SURFACE_CONTAINER_LOW     : (u32, u32) = ( 96, 10);
    pub const SURFACE_CONTAINER_LOWEST  : (u32, u32) = (  0,  4);
    pub const INVERSE_SURFACE           : (u32, u32) = ( 20, 90);
    pub const INVERSE_ON_SURFACE        : (u32, u32) = ( 95, 20);
    pub const OUTLINE                   : (u32, u32) = ( 50, 60);
    pub const OUTLINE_VARIANT           : (u32, u32) = ( 80, 30);
    pub const SCRIM                     : (u32, u32) = (  0,  0);
    pub const SHADOW                    : (u32, u32) = (  0,  0);
}

impl Theme {
    pub fn from_world(world: &World) -> Self {
        Theme::dark(world)
    }

    pub fn light(world: &World) -> Self {
        Theme::source(
            world,
            world.insert(Val::new(0.67)),
            world.insert(Val::new(false)),
        )
    }

    pub fn dark(world: &World) -> Self {
        Theme::source(
            world,
            world.insert(Val::new(0.67)),
            world.insert(Val::new(true)),
        )
    }

    pub fn source(world: &World, hue: Handle<Val<f32>>, dark: Handle<Val<bool>>) -> Self {
        Theme {
            color: ThemeColor::source(world, hue, dark),
            state: ThemeState::from_world(world),
            shape: ThemeShape::from_world(world),
            hue,
            dark,
            animation: 30.0,
        }
    }
}

impl ThemeColor {
    pub fn source(world: &World, hue: Handle<Val<f32>>, dark: Handle<Val<bool>>) -> Self {
        use consts::*;

        let bind = |(l_light, l_dark), c, h| {
            ValBind::new(world, move |world| {
                let hue = world.fetch(hue).val;
                let dark = world.fetch(dark).val;
                let l = match dark {
                    true => l_dark as f32 / 100.0,
                    false => l_light as f32 / 100.0,
                };
                // simple tweak to OkLab's L value (get it closer to CIEDE2000)
                let l_ = l.powf(0.73);
                let oklch = Oklch(Vec4::new(l_, c, hue + h, 1.0));
                oklch.to_oklab().to_srgb_linear()
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
            corner_radius: world.insert(Val::new(8.0)),
        }
    }
}

impl Element for Theme {}
