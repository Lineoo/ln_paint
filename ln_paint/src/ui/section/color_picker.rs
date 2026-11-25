use std::f32::consts::SQRT_2;

use glam::{IVec2, UVec2, Vec4};
use ln_world::{Element, Handle, World};

use crate::{
    color::{hsl::Hsl, oklab::Oklab},
    input::drag::{DragEvent, DragStatus},
    paint::wrapper::LayerPage,
    render::nest::{NestAttr, NestBuilder, NestRect},
    ui::{
        common::WidgetCommon,
        layout::transform::{Transform, TransformValue},
        object::color_snip::{ColorSnip, ColorSnipAttr},
        renderer::{rrect::RRect, svg::svg_render},
        section::{debug_panel::DebugPanel, layer_selection::LayerSelectionPanel},
        widget::{
            button::{Button, ButtonImage, ButtonTheme, ButtonToggle},
            container::Container,
            palette::{
                hsl::{ColorHsla, HslRing, HslSquare, INNER_RADIUS, SetColorHsla},
                oklab::{ColorOklab, OklabBar, OklabPolar, SetColorOklab},
            },
            tabs::{TabEntry, Tabs},
        },
    },
    unit::{FI64Ext, IRect},
    val::{Val, ValBind, ValChanged},
};

pub struct ColorPickerPanel {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for ColorPickerPanel {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("color_picker_panel");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let hsl_rect = world.insert(Val::new(IRect::default()));
        let hsl_visible = world.insert(Val::new(false));
        world.insert(LayerHslPalette {
            common: WidgetCommon {
                visible: hsl_visible,
                ..self.common
            },
            rect: hsl_rect,
            hsl: Hsl(Vec4::new(0.3, 0.5, 0.5, 1.0)),
        });

        let oklab_rect = world.insert(Val::new(IRect::default()));
        let oklab_visible = world.insert(Val::new(false));
        world.insert(LayerOklabPalette {
            common: WidgetCommon {
                visible: oklab_visible,
                ..self.common
            },
            rect: oklab_rect,
            oklab: Oklab(Vec4::new(0.0, 0.0, 0.0, 1.0)),
        });

        let layer_rect = world.insert(Val::new(IRect::default()));
        let layer_visible = world.insert(Val::new(false));
        world.insert(LayerSelectionPanel {
            common: WidgetCommon {
                visible: layer_visible,
                ..self.common
            },
            rect: layer_rect,
        });

        let debug_rect = world.insert(Val::new(IRect::default()));
        let debug_visible = world.insert(Val::new(false));
        world.insert(DebugPanel {
            common: WidgetCommon {
                visible: debug_visible,
                ..self.common
            },
            rect: debug_rect,
        });

        world.insert(Tabs {
            common: self.common,
            rect: self.rect,
            // TODO TabItem like SliderLabel
            tabs: vec![
                TabEntry {
                    icon: ButtonImage::new(
                        svg_render(include_bytes!("../../../res/interface/palette.svg")),
                        12,
                    ),
                    rect: hsl_rect,
                    visible: hsl_visible,
                },
                TabEntry {
                    icon: ButtonImage::new(
                        svg_render(include_bytes!("../../../res/interface/palette.svg")),
                        12,
                    ),
                    rect: oklab_rect,
                    visible: oklab_visible,
                },
                TabEntry {
                    icon: ButtonImage::new(
                        svg_render(include_bytes!("../../../res/interface/layers.svg")),
                        12,
                    ),
                    rect: layer_rect,
                    visible: layer_visible,
                },
                TabEntry {
                    icon: ButtonImage::new(
                        svg_render(include_bytes!("../../../res/interface/bug.svg")),
                        12,
                    ),
                    rect: debug_rect,
                    visible: debug_visible,
                },
            ],
            active: world.insert(Val::new(0)),
        });
    }
}

pub struct ColorPickerButton {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub selected: Handle<Val<bool>>,
}

impl Element for ColorPickerButton {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("color_picker_button");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.fetch(self.common.theme);
        let page = world.single_fetch::<LayerPage>();
        let button = world.insert(Button {
            common: self.common,
            rect: self.rect,
            theme: ButtonTheme::surface(&theme),
            image: None,
            selected: self.selected,
            hovering: world.insert(Val::new(false)),
        });
        world.insert(ButtonToggle { source: button });

        let icon_transform =
            TransformValue::anchor((0.5, 0.5), IRect::new_half(IVec2::ZERO, UVec2::splat(10)));
        let icon_rect = world.build(Transform::new(icon_transform, self.rect));
        let color = ValBind::new(world, |world| {
            let layer = world.single_fetch::<LayerPage>();
            layer.color().to_linear()
        })
        .listen(page.handle())
        .finish();

        world.insert(RRect {
            common: self.common,
            rect: icon_rect,
            order: 21,
            color,
            radius: 10.0,
            width: 0.0,
        });

        let mut dragging = None;
        world.observer(button, move |drag: &DragEvent, world| {
            let this = world.fetch(this);
            let page_camera = world.single_fetch::<LayerPage>().common.camera;
            let page_camera = world.fetch(page_camera);
            let page = world.single_fetch::<LayerPage>();
            let snip_rect = *dragging.get_or_insert_with(|| {
                let position = page_camera
                    .screen_to_world(drag.capture.pointer.screen)
                    .q32_round();
                let rect = world.insert(Val::new(IRect::new_half(position, UVec2::splat(24))));

                world.insert(ColorSnip {
                    common: WidgetCommon {
                        camera: page_camera.handle(),
                        ..this.common
                    },
                    rect,
                    attr: ColorSnipAttr {
                        position,
                        color: page.color(),
                    },
                });

                rect
            });

            let next = IRect::new_half(
                page_camera
                    .screen_to_world(drag.capture.pointer.screen)
                    .q32_round(),
                UVec2::splat(24),
            );
            world.queue(move |world| {
                let mut rect = world.fetch_mut(snip_rect);
                rect.val = next;
                world.queue_trigger(rect.handle(), ValChanged);
            });

            if drag.status == DragStatus::End {
                dragging = None;
            }
        });
    }
}

pub struct LayerHslPalette {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub hsl: Hsl,
}

impl Element for LayerHslPalette {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        const RING_HALF: u32 = 100;
        const SQUARE_HALF: u32 = (RING_HALF as f32 * INNER_RADIUS * SQRT_2) as u32;

        world.scope("palette_hsl");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let attr = NestAttr::default();
        let (container, inner, common) = world.build(NestBuilder {
            common: self.common,
            rect: self.rect,
            attr,
        });
        world.insert(Container {
            nest: container,
            theme: self.common.theme,
            restriction: inner,
        });

        let ring_rect = world.build(Transform::new(
            TransformValue::anchor(
                (0.5, 0.5),
                IRect::new_half(IVec2::ZERO, UVec2::splat(RING_HALF)),
            ),
            inner,
        ));
        let ring = world.insert(HslRing {
            common,
            rect: ring_rect,
            hsl: self.hsl,
            inner_radius: INNER_RADIUS,
        });

        let square_rect = world.build(Transform::new(
            TransformValue::anchor(
                (0.5, 0.5),
                IRect::new_half(IVec2::ZERO, UVec2::splat(SQUARE_HALF)),
            ),
            inner,
        ));
        let square = world.insert(HslSquare {
            common,
            rect: square_rect,
            hsl: self.hsl,
        });

        let layer = world.single::<LayerPage>();
        world.observer(ring, move |&ColorHsla(hsla), world| {
            let mut layer = world.fetch_mut(layer);
            layer.set_color(hsla.to_srgb());
            world.queue_trigger(layer.handle(), ValChanged);
        });
        world.observer(square, move |&ColorHsla(hsla), world| {
            let mut layer = world.fetch_mut(layer);
            layer.set_color(hsla.to_srgb());
            world.queue_trigger(layer.handle(), ValChanged);
        });
        world.observer(layer, move |&ValChanged, world| {
            let layer = world.fetch(layer);
            let hsla = Hsl::from_srgb(layer.color());
            world.trigger(ring, &SetColorHsla(hsla));
            world.trigger(square, &SetColorHsla(hsla));
        });
    }
}

pub struct LayerOklabPalette {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub oklab: Oklab,
}

impl Element for LayerOklabPalette {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("palette_oklab");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let attr = NestAttr::default();
        let (container, inner, common) = world.build(NestBuilder {
            common: self.common,
            rect: self.rect,
            attr,
        });
        world.insert(Container {
            nest: container,
            theme: self.common.theme,
            restriction: inner,
        });
        let local = world.build(NestRect { source: container });

        let polar_rect = world.build(Transform::new(
            TransformValue::anchor(
                (0.5, 0.5),
                IRect::new_half(IVec2::new(-30, 0), UVec2::splat(100)),
            ),
            local,
        ));
        let polar = world.insert(OklabPolar {
            common,
            rect: polar_rect,
            oklab: self.oklab,
        });

        let bar_rect = world.build(Transform::new(
            TransformValue::anchor(
                (0.5, 0.5),
                IRect::new_half(IVec2::new(110, 0), UVec2::new(20, 100)),
            ),
            local,
        ));
        let bar = world.insert(OklabBar {
            common,
            rect: bar_rect,
            oklab: self.oklab,
        });

        let layer = world.single::<LayerPage>();
        world.observer(polar, move |&ColorOklab(color), world| {
            let mut layer = world.fetch_mut(layer);
            layer.set_color(color.to_srgb_linear().to_srgb());
            world.queue_trigger(layer.handle(), ValChanged);
        });
        world.observer(bar, move |&ColorOklab(color), world| {
            let mut layer = world.fetch_mut(layer);
            layer.set_color(color.to_srgb_linear().to_srgb());
            world.queue_trigger(layer.handle(), ValChanged);
        });
        world.observer(layer, move |&ValChanged, world| {
            let layer = world.fetch(layer);
            let oklab = Oklab::from_srgb_linear(layer.color().to_linear());
            world.trigger(polar, &SetColorOklab(oklab));
            world.trigger(bar, &SetColorOklab(oklab));
        });
    }
}
