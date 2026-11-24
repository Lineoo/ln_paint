use cosmic_text::{Align, Metrics};
use glam::UVec2;
use ln_world::{Element, Handle, HandleGeneric, World};

use crate::{
    input::{collider::ToolRectCollider, pointer::PointerEvent},
    ui::{
        common::WidgetCommon,
        layout::anim::{Animator, AnimatorSnap},
        renderer::{
            rrect::RRect,
            text::{Text, TextAttr},
        },
        theme::{LegacyTheme, LegacyThemeVal},
    },
    unit::{Axis, FI64Ext, IRect},
    val::{Val, ValBind, ValChanged},
};

pub struct SliderValue(pub f32);

pub struct Slider {
    pub common: WidgetCommon,
    pub value: Handle<Val<f32>>,
    pub rect: Handle<Val<IRect>>,
    pub hover: Handle<Val<bool>>,
    pub axis: Axis,
    pub pressed: bool,
}

pub struct SliderLabel {
    pub common: WidgetCommon,
    pub text: Handle<Val<String>>,
    pub source: Handle<Slider>,
    pub clockwise: bool,
}

impl Element for Slider {
    fn when_insert(&mut self, world: &World, handle: Handle<Self>) {
        world.scope("slider");
        world.set_autodeps(Some(handle));
        world.dependency(handle, self.rect);
        world.dependency(handle, self.common.visible);

        let theme = world.single_fetch::<LegacyTheme>();
        let theme_val = world.single_fetch::<LegacyThemeVal>();
        let rect = world.fetch(self.rect).val;
        let value = world.fetch(self.value).val;

        let position = into_position(rect, self.axis, value);
        let back_rect_anim = world.insert(Val::new(back_rect(rect, self.axis)));
        let back_rect_val = world.build(Animator::new(back_rect_anim, theme.anim_factor));
        let front_rect_val = world.insert(Val::new(front_rect(rect, self.axis, position)));
        let knob_rect_val = world.insert(Val::new(knob_rect(rect, self.axis, position)));
        let knob_split_rect_val =
            world.insert(Val::new(knob_split_rect(rect, self.axis, position)));
        let pressed = world.insert(Val::new(false));

        let knob_color_anim = ValBind::new(world, move |world| {
            let theme = world.single_fetch::<LegacyThemeVal>();
            let pressed_val = world.fetch(pressed).val;
            match pressed_val {
                true => world.fetch(theme.gray1).val,
                false => world.fetch(theme.gray0).val,
            }
        })
        .listen(pressed)
        .listen(theme_val.gray0)
        .listen(theme_val.gray1)
        .finish();
        let knob_color_val = world.build(Animator::new(knob_color_anim, theme.anim_factor));

        let knob_split_color_anim = ValBind::new(world, move |world| {
            let theme = world.single_fetch::<LegacyThemeVal>();
            let pressed_val = world.fetch(pressed).val;
            match pressed_val {
                true => world.fetch(theme.theme).val,
                false => world.fetch(theme.gray1).val,
            }
        })
        .listen(pressed)
        .listen(theme_val.gray1)
        .listen(theme_val.theme)
        .finish();
        let knob_split_color_val =
            world.build(Animator::new(knob_split_color_anim, theme.anim_factor));

        world.insert(RRect {
            common: self.common,
            rect: back_rect_val,
            order: 20,
            color: theme_val.gray2,
            radius: theme.roundness,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: front_rect_val,
            order: 21,
            color: theme_val.theme,
            radius: theme.roundness,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: knob_rect_val,
            order: 22,
            color: knob_color_val,
            radius: theme.roundness,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: knob_split_rect_val,
            order: 23,
            color: knob_split_color_val,
            radius: 2.0,
            width: 0.0,
        });

        world.insert(ToolRectCollider {
            common: self.common,
            target: handle.untyped(),
            rect: self.rect,
            order: 10,
        });

        world.observer(handle, move |event: &PointerEvent, world| {
            let mut this = world.fetch_mut(handle);
            let rect = world.fetch(this.rect).val;

            match event {
                PointerEvent::Enter(_) | PointerEvent::Leave(_) => {
                    let hover = matches!(event, PointerEvent::Enter(_));
                    world.fetch_mut(this.hover).val = hover;
                    world.queue_trigger(this.hover, ValChanged);
                }
                PointerEvent::Press(capture)
                | PointerEvent::Move(capture)
                | PointerEvent::Release(capture) => {
                    let position = (capture.pointer.position.q32_round() - rect.center())
                        .dot(this.axis.direction());
                    let value = from_position(rect, this.axis, position);
                    world.queue_trigger(handle, SliderValue(value));

                    if matches!(event, PointerEvent::Press(_)) {
                        this.pressed = true;
                        let mut back = world.fetch_mut(back_rect_anim);
                        let mut pressed = world.fetch_mut(pressed);
                        back.val = back_rect_pressed(rect, this.axis);
                        pressed.val = true;
                        world.queue_trigger(back.handle(), ValChanged);
                        world.queue_trigger(pressed.handle(), ValChanged);
                    } else if matches!(event, PointerEvent::Release(_)) {
                        this.pressed = false;
                        let mut back = world.fetch_mut(back_rect_anim);
                        let mut pressed = world.fetch_mut(pressed);
                        back.val = back_rect(rect, this.axis);
                        pressed.val = false;
                        world.queue_trigger(back.handle(), ValChanged);
                        world.queue_trigger(pressed.handle(), ValChanged);
                    }
                }
                _ => {}
            }
        });

        world.observer(self.value, move |&ValChanged, world| {
            let this = world.fetch(handle);
            let value = world.fetch(this.value).val;
            let rect = world.fetch(this.rect).val;
            let mut front = world.fetch_mut(front_rect_val);
            let mut knob = world.fetch_mut(knob_rect_val);
            let mut knob_split = world.fetch_mut(knob_split_rect_val);

            let position = into_position(rect, this.axis, value);

            front.val = front_rect(rect, this.axis, position);
            knob_split.val = knob_split_rect(rect, this.axis, position);
            knob.val = match this.pressed {
                true => knob_rect_pressed(rect, this.axis, position),
                false => knob_rect(rect, this.axis, position),
            };

            world.queue_trigger(front.handle(), ValChanged);
            world.queue_trigger(knob_split.handle(), ValChanged);
            world.queue_trigger(knob.handle(), ValChanged);
        });

        world.observer(self.rect, move |ValChanged, world| {
            let this = world.fetch(handle);
            let value = world.fetch(this.value).val;
            let rect = world.fetch(this.rect).val;
            let mut front = world.fetch_mut(front_rect_val);
            let mut knob = world.fetch_mut(knob_rect_val);
            let mut knob_split = world.fetch_mut(knob_split_rect_val);
            let mut back = world.fetch_mut(back_rect_anim);

            let back_rect = back_rect(rect, this.axis);
            back.val = back_rect;

            let position = into_position(rect, this.axis, value);
            front.val = front_rect(rect, this.axis, position);
            knob_split.val = knob_split_rect(rect, this.axis, position);
            knob.val = match this.pressed {
                true => knob_rect_pressed(rect, this.axis, position),
                false => knob_rect(rect, this.axis, position),
            };

            world.queue_trigger(back.handle(), ValChanged);
            world.queue_trigger(back_rect_val, AnimatorSnap);
            world.queue_trigger(front.handle(), ValChanged);
            world.queue_trigger(knob_split.handle(), ValChanged);
            world.queue_trigger(knob.handle(), ValChanged);
        });
    }
}

impl Element for SliderLabel {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("slider_label");
        world.set_autodeps(Some(this));

        let source = world.fetch(self.source);
        let theme = world.single_fetch::<LegacyThemeVal>();
        let rect = world.fetch(source.rect).val;
        let value = world.fetch(source.value).val;

        let position = into_position(rect, source.axis, value);
        let label_rect = world.insert(Val::new(label_back_rect(
            rect,
            source.axis,
            self.clockwise,
            position,
        )));
        let common = WidgetCommon {
            camera: self.common.camera,
            visible: source.hover,
        };

        world.insert(RRect {
            common,
            rect: label_rect,
            order: 100,
            color: theme.gray0,
            radius: LABEL_HALF.y as f32,
            width: 0.0,
        });

        world.insert(Text {
            text: self.text,
            rect: label_rect,
            attr: TextAttr {
                metrics: Metrics::new(12., 2. * LABEL_HALF.y as f32),
                align: Align::Center,
                ..Default::default()
            },
            order: 150,
            common,
            modulate: theme.distinct,
            outdated: false,
        });

        world.observer(source.rect, move |ValChanged, world| {
            let this = world.fetch(this);
            let source = world.fetch(this.source);
            let rect = world.fetch(source.rect).val;
            let value = world.fetch(source.value).val;
            let mut label_rect = world.fetch_mut(label_rect);
            let position = into_position(rect, source.axis, value);
            label_rect.val = label_back_rect(rect, source.axis, this.clockwise, position);
            world.queue_trigger(label_rect.handle(), ValChanged);
        });

        world.observer(source.value, move |ValChanged, world| {
            let this = world.fetch(this);
            let source = world.fetch(this.source);
            let rect = world.fetch(source.rect).val;
            let value = world.fetch(source.value).val;
            let mut label_rect = world.fetch_mut(label_rect);
            let position = into_position(rect, source.axis, value);
            label_rect.val = label_back_rect(rect, source.axis, this.clockwise, position);
            world.queue_trigger(label_rect.handle(), ValChanged);
        });
    }
}

const BAR_EX: i32 = 5;
const BAR_HW: i32 = 6;
const KNOB_HALF: UVec2 = UVec2::new(16, 8);
const KNOB_HALF_PRESSED: UVec2 = UVec2::new(16, 7);
const KNOB_SPLIT_HALF: UVec2 = UVec2::new(2, 5);
const LABEL_GAP: i32 = 2;
const LABEL_HALF: UVec2 = UVec2::new(30, 12);

fn back_rect(rect: IRect, axis: Axis) -> IRect {
    IRect::new_half(
        rect.center(),
        axis.half(rect.axis_length(axis) / 2, BAR_HW as u32),
    )
}

fn back_rect_pressed(rect: IRect, axis: Axis) -> IRect {
    back_rect(rect, axis).expand(BAR_EX)
}

fn front_rect(rect: IRect, axis: Axis, position: i32) -> IRect {
    let length = position + rect.axis_length(axis) as i32 / 2;
    IRect::new_half(
        rect.center() + axis.direction() * (position - length / 2),
        axis.half((length as u32).div_ceil(2), BAR_HW as u32),
    )
}

fn knob_rect(rect: IRect, axis: Axis, position: i32) -> IRect {
    knob_half_rect(rect, axis, KNOB_HALF, position)
}

fn knob_rect_pressed(rect: IRect, axis: Axis, position: i32) -> IRect {
    knob_half_rect(rect, axis, KNOB_HALF_PRESSED, position)
}

fn knob_split_rect(rect: IRect, axis: Axis, position: i32) -> IRect {
    knob_half_rect(rect, axis, KNOB_SPLIT_HALF, position)
}

fn knob_half_rect(rect: IRect, axis: Axis, half: UVec2, position: i32) -> IRect {
    IRect::new_half(rect.center(), axis.half(half.x, half.y)) + axis.direction() * position
}

fn into_position(rect: IRect, axis: Axis, value: f32) -> i32 {
    let reach = knob_reach(rect, axis, KNOB_HALF.x);
    ((value - 0.5) * (reach * 2) as f32).round() as i32
}

fn from_position(rect: IRect, axis: Axis, position: i32) -> f32 {
    let reach = knob_reach(rect, axis, KNOB_HALF.x);
    (position as f32 / (reach * 2) as f32 + 0.5).clamp(0.0, 1.0)
}

fn knob_reach(rect: IRect, axis: Axis, half: u32) -> i32 {
    rect.axis_length(axis) as i32 / 2 - half as i32
}

fn label_back_rect(rect: IRect, axis: Axis, clockwise: bool, position: i32) -> IRect {
    let raxis = match clockwise {
        true => axis.rotate_cw(),
        false => axis.rotate_ccw(),
    };
    let half = match raxis.is_horizontal() {
        true => LABEL_HALF.x,
        false => LABEL_HALF.y,
    } as i32;
    IRect::new_half(rect.center(), LABEL_HALF)
        + axis.direction() * position
        + raxis.direction() * (BAR_HW + BAR_EX + LABEL_GAP + half)
}
