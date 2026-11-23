use glam::{IVec2, UVec2, Vec4};
use ln_world::{Element, Handle, HandleGeneric, World};

use crate::{
    color::srgb::{mul_alpha, srgb_gamma_decode},
    input::{collider::ToolRectCollider, drag::Drag, mouse::MouseMenu, pointer::PointerEvent},
    paint::{input::LayerDrag, wrapper::LayerPage},
    ui::{
        common::WidgetCommon,
        layout::{
            anim::Animator,
            transform::{Transform, TransformValue},
        },
        renderer::{canvas::Canvas, rrect::RRect, svg::svg_render},
        theme::{Theme, ThemeVal},
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
    pub color: Vec4,
}

impl Element for ColorSnip {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("color_snip");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.single_fetch::<Theme>();
        let theme_val = world.single_fetch::<ThemeVal>();
        let input = world.single::<LayerDrag>();
        let visible = self.common.visible;
        let base = mul_alpha(srgb_gamma_decode(self.attr.color));

        let hover = world.insert(Val::new(false));
        let pressed = world.insert(Val::new(false));

        let shadow_offset = theme.shadow_offset.as_ivec2();
        let shadow_rect = world.build(Transform::new(
            TransformValue::shift(shadow_offset.x, shadow_offset.y),
            self.rect,
        ));
        world.insert(RRect {
            common: self.common,
            rect: shadow_rect,
            order: 9,
            color: theme_val.shadow_color,
            radius: theme.roundness,
            width: theme.shadow_blur,
        });

        let color = ValBind::new(world, move |world| match world.fetch(pressed).val {
            true => base * 0.7 + Vec4::new(0.2, 0.2, 0.2, 0.3) * base.w,
            false => base,
        })
        .listen(pressed)
        .finish();
        let color = world.build(Animator::new(color, theme.anim_factor));

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: 10,
            color,
            radius: theme.roundness,
            width: 0.0,
        });

        let distinct = theme_val.distinct;
        let icon_modulate = ValBind::new(world, move |world| {
            let hover = world.fetch(hover).val;
            world
                .fetch(distinct)
                .val
                .with_w(if hover { 1.0 } else { 0.0 })
        })
        .listen(hover)
        .listen(distinct)
        .finish();
        let icon_modulate = world.build(Animator::new(icon_modulate, theme.anim_factor));
        let icon = svg_render(include_bytes!("../../../res/interface/pipette.svg"));
        world.insert(Canvas {
            common: WidgetCommon {
                camera: self.common.camera,
                visible,
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
        let mut source = None;
        world.observer(this, move |event: &PointerEvent, world| match event {
            PointerEvent::Enter(_) | PointerEvent::Leave(_) => {
                let value = matches!(event, PointerEvent::Enter(_));
                world.fetch_mut(hover).val = value;
                world.queue_trigger(hover, ValChanged);
            }
            PointerEvent::Press(_) => {
                source = Some(world.fetch(world.fetch(this).rect).val.center());
                world.fetch_mut(pressed).val = true;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Move(_) => {
                if let Some(event) = drag.update(event) {
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
                }
            }
            PointerEvent::Release(_) => {
                let dragged = drag.update(event).is_some();
                if !dragged {
                    let mut page = world.single_fetch_mut::<LayerPage>();
                    page.set_color(world.fetch(this).attr.color);
                    world.queue_trigger(page.handle(), ValChanged);
                }
                source = None;
                world.fetch_mut(pressed).val = false;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Scroll { .. } => world.queue_trigger(input, event.clone()),
            PointerEvent::Hover(_) => {}
        });

        world.observer(this, move |&MouseMenu(_), world| {
            world.queue(move |world| {
                world.remove(this);
            });
        });
    }
}
