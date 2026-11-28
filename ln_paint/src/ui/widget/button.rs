use lnworld::{Element, Handle, HandleGeneric, World};

use crate::{
    color::srgb::{Srgb, SrgbLinear},
    input::{collider::ToolRectCollider, drag::Drag, hold::Hold, pointer::PointerEvent},
    ui::{
        common::WidgetCommon,
        layout::transform::{Transform, TransformValue},
        renderer::{
            canvas::Canvas,
            rrect::RRect,
            svg::PixImage,
            text::{Text, TextAttr},
        },
        theme::Theme,
    },
    unit::IRect,
    val::{Val, ValBind, ValChanged},
};

pub struct Button {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub theme: ButtonTheme,
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

pub struct ButtonImage {
    pub common: WidgetCommon,
    pub source: Handle<Button>,
    pub image: Handle<Val<PixImage>>,
    pub transform: TransformValue,
}

pub struct ButtonText {
    pub common: WidgetCommon,
    pub source: Handle<Button>,
    pub text: Handle<Val<String>>,
    pub transform: TransformValue,
    pub attr: TextAttr,
}

pub struct ButtonImageText {
    pub common: WidgetCommon,
    pub source: Handle<Button>,
    pub image: Handle<Val<PixImage>>,
    pub text: Handle<Val<String>>,
    pub image_transform: TransformValue,
    pub text_transform: TransformValue,
    pub attr: TextAttr,
}

pub struct ButtonToggle {
    pub source: Handle<Button>,
}

pub struct ButtonClick;
pub struct ButtonHold;

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

        let mut drag = Drag::default();
        let hold = Hold::default();
        let mut dragging = false;
        let mut held = false;
        world.observer(this, move |event: &PointerEvent, world| match event {
            PointerEvent::Enter(_) | PointerEvent::Leave(_) => {
                let hover = matches!(event, PointerEvent::Enter(_));
                world.fetch_mut(hovering).val = hover;
                world.queue_trigger(hovering, ValChanged);
            }
            PointerEvent::Press(_) => {
                dragging = false;
                held = false;
                world.fetch_mut(pressed).val = true;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Move(_) => {
                if held {
                    return;
                }
                if let Some(event) = drag.update(event) {
                    dragging = true;
                    world.queue_trigger(this, event);
                } else if hold.reached(event) {
                    held = true;
                    world.queue_trigger(this, ButtonHold);
                }
            }
            PointerEvent::Release(_) => {
                if !dragging && !held {
                    world.queue_trigger(this, ButtonClick);
                }
                if !held && let Some(event) = drag.update(event) {
                    world.queue_trigger(this, event);
                }
                held = false;
                world.fetch_mut(pressed).val = false;
                world.queue_trigger(pressed, ValChanged);
            }
            PointerEvent::Hover(_) | PointerEvent::Scroll { .. } | PointerEvent::Menu(_) => {}
        });
    }
}

impl Element for ButtonImage {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("button_image");
        world.set_autodeps(Some(this));

        let button = world.fetch(self.source);
        let bmp = world.fetch(self.image);
        let rect = world.build(Transform::new(self.transform, button.rect));
        world.insert(Canvas {
            common: self.common,
            rect,
            order: 10,
            modulate: button.theme.on_color,
            // FIXME deep clone
            data: bmp.val.bytes.clone(),
            data_width: bmp.val.bytes_width,
            data_height: bmp.val.bytes_height,
        });
    }
}

impl Element for ButtonText {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("button_text");
        world.set_autodeps(Some(this));

        let button = world.fetch(self.source);
        world.insert(Text {
            text: self.text,
            rect: world.build(Transform::new(self.transform, button.rect)),
            attr: self.attr.clone(),
            order: 10,
            common: self.common,
            modulate: button.theme.on_color,
            outdated: false,
        });
    }
}

impl Element for ButtonImageText {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("button_image_text");
        world.set_autodeps(Some(this));

        world.insert(ButtonImage {
            common: self.common,
            source: self.source,
            image: self.image,
            transform: self.image_transform,
        });
        world.insert(ButtonText {
            common: self.common,
            source: self.source,
            text: self.text,
            transform: self.text_transform,
            attr: self.attr.clone(),
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
