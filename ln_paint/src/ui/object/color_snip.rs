use cosmic_text::{Align, Attrs, Family, Metrics};
use glam::{DVec2, IVec2, UVec2, Vec4};
use ln_world::{Element, Handle, HandleGeneric, World};

use crate::{
    color::{
        oklab::Oklab,
        srgb::{Srgb, SrgbLinear},
    },
    input::{collider::ToolRectCollider, drag::Drag, hold::Hold, pointer::PointerEvent},
    lnwin::RootCamera,
    paint::{input::LayerDrag, wrapper::LayerPage},
    res::Icon,
    ui::{
        common::WidgetCommon,
        i18n::tr,
        layout::transform::{Transform, TransformValue},
        renderer::{canvas::Canvas, rrect::RRect, svg::svg_render, text::TextAttr},
        widget::{
            button::{Button, ButtonClick, ButtonImageText, ButtonTheme},
            panel::{Panel, PanelAttr},
        },
    },
    unit::{FI64Ext, IRect},
    val::{Val, ValBind, ValChanged},
};

pub struct ColorSnip {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub attr: ColorSnipAttr,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ColorSnipAttr {
    pub position: IVec2,
    pub color: Srgb,
}

struct ColorSnipMenu {
    common: WidgetCommon,
    source: Handle<ColorSnip>,
    position: IVec2,
}

fn open_menu(world: &World, source: Handle<ColorSnip>, screen: DVec2) -> Handle<ColorSnipMenu> {
    let root = world.single_fetch::<RootCamera>().0;
    let position = world.fetch(root).screen_to_world(screen).q32_floor();
    let common = WidgetCommon {
        camera: root,
        ..world.fetch(source).common
    };
    let menu = world.insert(ColorSnipMenu {
        common,
        source,
        position,
    });
    world.dependency(menu, source);
    menu
}

impl Element for ColorSnip {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("color_snip");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.fetch(self.common.theme);
        let input = world.single::<LayerDrag>();
        let visible = self.common.visible;

        let hover = world.insert(Val::new(false));
        let pressed = world.insert(Val::new(false));

        let shadow_offset = IVec2::ZERO;
        let shadow_rect = world.build(Transform::new(
            TransformValue::shift(shadow_offset.x, shadow_offset.y),
            self.rect,
        ));
        world.insert(RRect {
            common: self.common,
            rect: shadow_rect,
            order: 9,
            color: theme.color.shadow,
            radius: world.fetch(theme.shape.corner_radius).val,
            width: world.fetch(theme.shape.shadow_width).val,
        });

        let color = ValBind::new(world, move |world| {
            let color = world.fetch(this).attr.color.to_linear();
            let oklab = Oklab::from_srgb_linear(color);
            let on_color = match oklab.0.x > 0.5 {
                true => SrgbLinear(Vec4::W),
                false => SrgbLinear(Vec4::ONE),
            };
            match world.fetch(pressed).val {
                true => color.mix(on_color, 0.3),
                false => color,
            }
        })
        .listen(pressed)
        .animate(theme.animation);

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: 10,
            color,
            radius: world.fetch(theme.shape.corner_radius).val,
            width: 0.0,
        });

        let icon_modulate = ValBind::new(world, move |world| {
            let color = world.fetch(this).attr.color.to_linear();
            let oklab = Oklab::from_srgb_linear(color);
            let hover = world.fetch(hover).val;
            let on_color = match oklab.0.x > 0.5 {
                true => SrgbLinear(Vec4::W),
                false => SrgbLinear(Vec4::ONE),
            };
            let opacity = match hover {
                true => 1.0,
                false => 0.0,
            };
            on_color * opacity
        })
        .listen(hover)
        .animate(theme.animation);
        let icon = svg_render(Icon::PIPETTE);
        world.insert(Canvas {
            common: WidgetCommon {
                visible,
                ..self.common
            },
            rect: world.build(Transform::new(
                TransformValue::anchor((0.5, 0.5), IRect::new_half(IVec2::ZERO, UVec2::splat(12))),
                self.rect,
            )),
            order: 11,
            modulate: icon_modulate,
            data: icon.bytes,
            data_width: icon.bytes_width,
            data_height: icon.bytes_height,
        });

        world.insert(ToolRectCollider {
            common: self.common,
            target: this.untyped(),
            rect: self.rect,
            order: 10,
        });

        let mut drag = Drag::default();
        let hold = Hold::default();
        let mut held = false;
        let mut menu = None;
        let mut source = None;
        world.observer(this, move |event: &PointerEvent, world| match event {
            PointerEvent::Enter(_) | PointerEvent::Leave(_) => {
                let value = matches!(event, PointerEvent::Enter(_));
                world.fetch_mut(hover).val = value;
                world.queue_trigger(hover, ValChanged);
            }
            PointerEvent::Press(_) => {
                world.insert(hold.task());
                held = false;
                menu = None;
                source = Some(world.fetch(world.fetch(this).rect).val.center());
                world.fetch_mut(pressed).val = true;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Move(_) => {
                if let Some(event) = drag.update(event) {
                    if held {
                        held = false;
                        if let Some(menu) = menu.take() {
                            world.queue(move |world| {
                                world.remove(menu);
                            });
                        }
                    }
                    let this = world.fetch(this);
                    let mut rect = world.fetch_mut(this.rect);
                    let source = source.get_or_insert(rect.val.center());
                    rect.val = IRect::new_half(
                        *source
                            + (event.capture.pointer.position - event.capture.start.position)
                                .q32_round(),
                        UVec2::splat(24),
                    );
                    world.queue_trigger(rect.handle(), ValChanged);
                } else if !held && hold.reached(event) {
                    held = true;
                    menu = Some(open_menu(world, this, event.pointer().screen));
                }
            }
            PointerEvent::Release(_) => {
                let dragged = drag.update(event).is_some();
                if !held && !dragged {
                    let mut page = world.single_fetch_mut::<LayerPage>();
                    page.set_color(world.fetch(this).attr.color);
                    world.queue_trigger(page.handle(), ValChanged);
                }
                held = false;
                menu = None;
                source = None;
                world.fetch_mut(pressed).val = false;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Scroll { .. } => world.queue_trigger(input, event.clone()),
            PointerEvent::Menu(pointer) => {
                open_menu(world, this, pointer.screen);
            }
            PointerEvent::Hover(_) => {}
        });
    }
}

impl Element for ColorSnipMenu {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("color_snip_menu");
        world.set_autodeps(Some(this));
        world.dependency(this, self.source);

        let source = self.source;
        let theme = world.fetch(self.common.theme);
        let rect = world.insert(Val::new(IRect::from_anchor_extend(
            self.position,
            DVec2::new(0.0, 1.0),
            UVec2::new(128, 48),
        )));
        let button_rect = world.build(Transform::new(TransformValue::shrink(4, 4), rect));
        let fullscreen = world.insert(Val::new(IRect::FULLSCREEN));
        world.insert(ToolRectCollider {
            common: self.common,
            target: this.untyped(),
            rect: fullscreen,
            order: 10,
        });

        world.insert(Panel {
            common: self.common,
            rect,
            attr: PanelAttr {
                round: true,
                shadow: true,
            },
        });

        let button = world.insert(Button {
            common: self.common,
            rect: button_rect,
            theme: ButtonTheme::surface(&theme),
            selected: world.insert(Val::new(false)),
            hovering: world.insert(Val::new(false)),
        });

        world.insert(ButtonImageText {
            common: self.common,
            source: button,
            image: world.insert(Val::new(svg_render(Icon::TRASH))),
            text: world.insert(Val::new(tr("menu.delete").to_string())),
            image_transform: TransformValue::anchor(
                (0.0, 0.5),
                IRect::new_half(IVec2::new(20, 0), UVec2::splat(8)),
            ),
            text_transform: TransformValue::anchor(
                (0.0, 0.5),
                IRect::new_half(IVec2::new(80, 0), UVec2::new(40, 10)),
            ),
            attr: TextAttr {
                metrics: Metrics::new(15.0, 20.0),
                attrs: Attrs::new().family(Family::SansSerif),
                align: Align::Left,
            },
        });

        world.observer(this, move |event: &PointerEvent, world| {
            if matches!(event, PointerEvent::Release(_) | PointerEvent::Menu(_)) {
                world.queue(move |world| {
                    world.remove(this);
                });
            }
        });

        world.observer(button, move |&ButtonClick, world| {
            world.queue(move |world| {
                world.remove(source);
            });
        });
    }
}
