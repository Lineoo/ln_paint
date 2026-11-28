use std::sync::Arc;

use cosmic_text::{Align, Attrs, Family, Metrics};
use glam::Vec4;
use lnpaint::{
    color::{hsl::Hsl, oklab::Oklab},
    lnwin::Lnwin,
    res::Icon,
    ui::{
        common::WidgetCommon,
        layout::{
            luni::{
                LuniAlign, LuniAxis, LuniChild, LuniChildTemplate, LuniFlex, LuniParent, LuniRect,
            },
            transform::TransformValue,
        },
        renderer::{
            rrect::RRect,
            svg::svg_render,
            text::{Text, TextAttr},
        },
        widget::{
            button::{Button, ButtonImage, ButtonTheme},
            palette::{hsl::HslSquare, oklab::OklabPolar},
            slider::Slider,
        },
    },
    unit::{Axis, IRect},
    val::Val,
};
use lnworld::{Element, Handle, World};
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();

    let lnwin = Lnwin {
        payload: Some(Arc::new(|world, common, rect| {
            world.insert(WidgetShowcase { common, rect });
        })),
        ..Default::default()
    };

    let event_loop = EventLoop::builder().build().unwrap();
    event_loop.run_app(lnwin).unwrap();
}

pub struct WidgetShowcase {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for WidgetShowcase {
    fn when_insert(&mut self, world: &World, this: lnworld::Handle<Self>) {
        world.scope("widget_showcase");
        world.set_autodeps(Some(this));
        world.dependency(this, self.common.visible);

        let theme = world.fetch(self.common.theme);

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: -200,
            color: theme.color.surface,
            radius: 0.0,
            width: 0.0,
        });

        LuniFlex::new(
            self.rect,
            LuniParent {
                axis: LuniAxis::Row,
                gap: 4,
                padding: LuniRect {
                    left: 28,
                    bottom: 12,
                    right: 28,
                    top: 12,
                },
                ..Default::default()
            },
        )
        .scope(world, |luni| {
            LuniFlex::new(
                world.build(luni.child(LuniChild {
                    basis: Some(600),
                    ..Default::default()
                })),
                LuniParent {
                    axis: LuniAxis::Column,
                    gap: 4,
                    ..Default::default()
                },
            )
            .scope(world, |luni| {
                let t = |luni: &mut LuniFlex, s: &str, height, size| {
                    world.insert(Text {
                        text: world.insert(Val::new(String::from(s))),
                        rect: world.build(luni.child(LuniChild {
                            basis: Some(height),
                            ..Default::default()
                        })),
                        attr: TextAttr {
                            metrics: Metrics::new(size, height as f32),
                            attrs: Attrs::new().family(Family::SansSerif),
                            align: Align::Left,
                        },
                        order: 100,
                        common: self.common,
                        modulate: theme.color.on_surface,
                        outdated: false,
                    })
                };

                t(luni, "Widget Showcase", 34, 20.0);
                t(luni, "Buttons", 28, 12.0);

                LuniFlex::new(
                    world.build(luni.child(LuniChild {
                        basis: Some(24),
                        ..Default::default()
                    })),
                    LuniParent {
                        axis: LuniAxis::Row,
                        gap: 4,
                        template: LuniChildTemplate {
                            basis: 60,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )
                .scope(world, |luni| {
                    let mut b = |color, on_color| {
                        let button = world.insert(Button {
                            common: self.common,
                            rect: world.build(luni.child(LuniChild::default())),
                            theme: ButtonTheme {
                                color,
                                on_color,
                                ..ButtonTheme::surface(&theme)
                            },
                            selected: world.insert(Val::new(false)),
                            hovering: world.insert(Val::new(false)),
                        });

                        world.insert(ButtonImage {
                            common: self.common,
                            source: button,
                            image: world.insert(Val::new(svg_render(Icon::PEN))),
                            transform: TransformValue::center(8),
                        });
                    };

                    let c = &theme.color;
                    b(c.surface, c.on_surface);
                    b(c.primary, c.on_primary);
                    b(c.primary_container, c.on_primary_container);
                    b(c.secondary, c.on_secondary);
                    b(c.secondary_container, c.on_secondary_container);
                    b(c.tertiary, c.on_tertiary);
                    b(c.tertiary_container, c.on_tertiary_container);
                });

                t(luni, "Sliders", 28, 12.0);

                world.insert(Slider {
                    common: self.common,
                    value: world.insert(Val::new(0.5)),
                    rect: world.build(luni.child(LuniChild {
                        cross: Some(400),
                        align: Some(LuniAlign::FlexStart),
                        ..Default::default()
                    })),
                    hover: world.insert(Val::new(false)),
                    pressed: world.insert(Val::new(false)),
                    axis: Axis::Right,
                });

                t(luni, "Palettes", 28, 12.0);

                world.insert(OklabPolar {
                    common: self.common,
                    rect: world.build(luni.child(LuniChild {
                        basis: Some(180),
                        cross: Some(180),
                        align: Some(LuniAlign::FlexStart),
                        ..Default::default()
                    })),
                    oklab: Oklab(Vec4::new(0.80, -0.07, 0.02, 1.00)),
                });

                world.insert(HslSquare {
                    common: self.common,
                    rect: world.build(luni.child(LuniChild {
                        basis: Some(180),
                        cross: Some(180),
                        align: Some(LuniAlign::FlexStart),
                        ..Default::default()
                    })),
                    hsl: Hsl(Vec4::new(0.50, 0.70, 0.20, 1.00)),
                });
            });

            LuniFlex::new(
                world.build(luni.child(LuniChild {
                    basis: Some(600),
                    ..Default::default()
                })),
                LuniParent {
                    axis: LuniAxis::Row,
                    gap: 4,
                    ..Default::default()
                },
            )
            .scope(world, |luni| {
                world.insert(Slider {
                    common: self.common,
                    value: world.insert(Val::new(0.5)),
                    rect: world.build(luni.child(LuniChild {
                        cross: Some(200),
                        align: Some(LuniAlign::FlexStart),
                        ..Default::default()
                    })),
                    hover: world.insert(Val::new(false)),
                    pressed: world.insert(Val::new(false)),
                    axis: Axis::Up,
                });
            });
        });
    }
}
