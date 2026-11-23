use glam::{IVec2, UVec2, Vec4};
use ln_world::{Element, Handle, HandleGeneric, World};

use crate::{
    input::{collider::ToolRectCollider, drag::Drag, pointer::PointerEvent},
    ui::{
        common::WidgetCommon,
        layout::{
            anim::Animator,
            transform::{Transform, TransformValue},
        },
        renderer::{canvas::Canvas, rrect::RRect, svg::PixImage},
        theme::Theme,
    },
    unit::IRect,
    val::{Val, ValBind, ValChanged},
};

pub struct Button {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub theme: ButtonTheme,
    pub image: Option<ButtonImage>,
    pub selected: Handle<Val<bool>>,
    pub hovering: Handle<Val<bool>>,
}

pub struct ButtonTheme {
    pub idle_color: Handle<Val<Vec4>>,
    pub hover_color: Handle<Val<Vec4>>,
    pub press_color: Handle<Val<Vec4>>,
    pub selected_color: Handle<Val<Vec4>>,
}

#[derive(Clone)]
pub struct ButtonImage {
    pub transform: TransformValue,
    pub modulate: Handle<Val<Vec4>>,
    pub bmp: PixImage,
}

pub struct ButtonToggle {
    pub source: Handle<Button>,
}

pub struct ButtonClick;

impl ButtonImage {
    pub fn new(bmp: PixImage, modulate: Handle<Val<Vec4>>, size: u32) -> Self {
        ButtonImage {
            transform: TransformValue::anchor(
                (0.5, 0.5),
                IRect::new_half(IVec2::ZERO, UVec2::splat(size)),
            ),
            modulate,
            bmp,
        }
    }
}

impl Element for Button {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("button");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.single_fetch::<Theme>();

        let selected = self.selected;
        let hovering = self.hovering;
        let idle_color = self.theme.idle_color;
        let hover_color = self.theme.hover_color;
        let press_color = self.theme.press_color;
        let selected_color = self.theme.selected_color;

        let pressed = world.insert(Val::new(false));
        let frame_color_anim = ValBind::new(world, move |world| {
            let selected_val = world.fetch(selected).val;
            let hovering_val = world.fetch(hovering).val;
            let pressed_val = world.fetch(pressed).val;
            if selected_val {
                world.fetch(selected_color).val
            } else if pressed_val {
                world.fetch(press_color).val
            } else if hovering_val {
                world.fetch(hover_color).val
            } else {
                world.fetch(idle_color).val
            }
        })
        .listen(selected)
        .listen(hovering)
        .listen(pressed)
        .listen(idle_color)
        .listen(hover_color)
        .listen(press_color)
        .listen(selected_color)
        .finish();
        let frame_color = world.build(Animator::new(frame_color_anim, theme.anim_factor));

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: 10,
            color: frame_color,
            radius: theme.roundness,
            width: 0.0,
        });

        world.insert(ToolRectCollider {
            common: self.common,
            target: this.untyped(),
            rect: self.rect,
            order: 10,
        });

        if let Some(image) = &self.image {
            let icon_rect = world.build(Transform::new(image.transform, self.rect));
            world.insert(Canvas {
                common: self.common,
                rect: icon_rect,
                order: 11,
                modulate: image.modulate,
                data: image.bmp.bytes.clone(),
                data_width: image.bmp.bytes_width,
                data_height: image.bmp.bytes_height,
            });
        }

        let mut drag = Drag::default();
        let mut dragging = false;
        world.observer(this, move |event: &PointerEvent, world| match event {
            PointerEvent::Enter(_) | PointerEvent::Leave(_) => {
                let hover = matches!(event, PointerEvent::Enter(_));
                world.fetch_mut(hovering).val = hover;
                world.queue_trigger(hovering, ValChanged);
            }
            PointerEvent::Press(_) => {
                dragging = false;
                world.fetch_mut(pressed).val = true;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Move(_) => {
                if let Some(event) = drag.update(event) {
                    dragging = true;
                    world.trigger(this, &event);
                }
            }
            PointerEvent::Release(_) => {
                if !dragging {
                    world.queue_trigger(this, ButtonClick);
                }
                if let Some(event) = drag.update(event) {
                    world.trigger(this, &event);
                }
                world.fetch_mut(pressed).val = false;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Hover(_) | PointerEvent::Scroll { .. } => {}
        });
    }
}

impl Element for ButtonToggle {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("button_toggle");
        world.set_autodeps(Some(this));

        world.observer(self.source, move |ButtonClick, world| {
            let this = world.fetch(this);
            let button = world.fetch(this.source);
            let mut selected = world.fetch_mut(button.selected);
            selected.val = !selected.val;
            world.queue_trigger(selected.handle(), ValChanged);
        });
    }
}
