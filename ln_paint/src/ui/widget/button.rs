use glam::{IVec2, UVec2};
use ln_world::{Element, Handle, HandleGeneric, World};

use crate::{
    color::srgb::{Srgb, SrgbLinear},
    input::{collider::ToolRectCollider, drag::Drag, pointer::PointerEvent},
    ui::{
        common::WidgetCommon,
        layout::transform::{Transform, TransformValue},
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
    pub color: Handle<Val<SrgbLinear>>,
    pub on_color: Handle<Val<SrgbLinear>>,
    pub active_color: Handle<Val<SrgbLinear>>,
    pub active_on_color: Handle<Val<SrgbLinear>>,
    pub hover: Handle<Val<f32>>,
    pub press: Handle<Val<f32>>,
    pub focus: Handle<Val<f32>>,
    pub corner_radius: Handle<Val<f32>>,
}

// TODO use SliderLabel style
#[derive(Clone)]
pub struct ButtonImage {
    pub transform: TransformValue,
    pub bmp: PixImage,
}

pub struct ButtonToggle {
    pub source: Handle<Button>,
}

pub struct ButtonClick;

impl ButtonTheme {
    pub fn surface(theme: &Theme) -> Self {
        ButtonTheme {
            color: theme.color.surface,
            on_color: theme.color.on_surface,
            active_color: theme.color.primary_container,
            active_on_color: theme.color.on_primary_container,
            hover: theme.state.hover,
            press: theme.state.press,
            focus: theme.state.focus,
            corner_radius: theme.shape.corner_radius,
        }
    }
}

impl ButtonImage {
    pub fn new(bmp: PixImage, size: u32) -> Self {
        ButtonImage {
            transform: TransformValue::anchor(
                (0.5, 0.5),
                IRect::new_half(IVec2::ZERO, UVec2::splat(size)),
            ),
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

        let theme = world.fetch(self.common.theme);

        let selected = self.selected;
        let hovering = self.hovering;
        let pressed = world.insert(Val::new(false));
        let radius = world.fetch(theme.shape.corner_radius).val;

        let color = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            let selected = world.fetch(selected).val;
            let pressed = world.fetch(pressed).val;
            let hovering = world.fetch(hovering).val;

            let color = match selected {
                true => world.fetch(this.theme.active_color).val,
                false => world.fetch(this.theme.color).val,
            };

            let on_color = match selected {
                true => world.fetch(this.theme.active_on_color).val,
                false => world.fetch(this.theme.on_color).val,
            };

            let factor = if pressed {
                world.fetch(this.theme.press).val
            } else if hovering {
                world.fetch(this.theme.hover).val
            } else {
                0.0
            };

            Srgb::mix(color.to_srgb(), on_color.to_srgb(), factor).to_linear()
        })
        .listen(selected)
        .listen(hovering)
        .listen(pressed)
        .listen(self.theme.color)
        .listen(self.theme.on_color)
        .listen(self.theme.active_color)
        .listen(self.theme.active_on_color)
        .listen(self.theme.press)
        .listen(self.theme.hover)
        .animate(theme.animation);

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: 10,
            color,
            radius,
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
                modulate: self.theme.on_color,
                // FIXME: deep clone
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
