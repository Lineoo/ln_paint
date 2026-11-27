use glam::{IVec2, UVec2};
use ln_world::{Element, Handle, HandleGeneric, World};

use crate::{
    input::collider::ToolRectCollider,
    ui::{
        common::WidgetCommon,
        layout::{
            luni::{
                LuniAlign, LuniAxis, LuniChild, LuniChildTemplate, LuniDistribution, LuniFlex,
                LuniParent, LuniRect,
            },
            transform::{Transform, TransformEdge, TransformValue},
        },
        renderer::{rrect::RRect, svg::PixImage},
        widget::button::{Button, ButtonClick, ButtonImage, ButtonTheme},
    },
    unit::IRect,
    val::{Val, ValBind, ValChanged},
};

pub struct Tabs {
    pub rect: Handle<Val<IRect>>,
    pub common: WidgetCommon,
    pub tabs: Vec<TabEntry>,
    pub active: Handle<Val<usize>>,
}

pub struct TabEntry {
    pub icon: Handle<Val<PixImage>>,
    pub rect: Handle<Val<IRect>>,
    pub visible: Handle<Val<bool>>,
}

impl Element for Tabs {
    fn when_insert(&mut self, world: &World, handle: Handle<Self>) {
        world.scope("tabs");
        world.set_autodeps(Some(handle));
        world.dependency(handle, self.rect);
        world.dependency(handle, self.common.visible);

        let theme = world.fetch(self.common.theme);

        let roundness = world.fetch(theme.shape.corner_radius).val;
        let shadow_offset = IVec2::ZERO;
        let shadow_blur = world.fetch(theme.shape.shadow_width).val;

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: -10,
            color: theme.color.surface_container,
            radius: roundness,
            width: 0.0,
        });

        let shadow_rect = world.build(Transform::new(
            TransformValue::shift(shadow_offset.x, shadow_offset.y),
            self.rect,
        ));
        world.insert(RRect {
            common: self.common,
            rect: shadow_rect,
            order: -11,
            color: theme.color.shadow,
            radius: roundness,
            width: shadow_blur,
        });

        world.insert(ToolRectCollider {
            common: self.common,
            target: handle.untyped(),
            rect: self.rect,
            order: -10,
        });

        let side_rect = world.build(Transform::new(side_transform(), self.rect));
        let main_rect = world.build(Transform::new(main_transform(), self.rect));

        let mut luni = LuniFlex::new(
            side_rect,
            LuniParent {
                axis: LuniAxis::Column,
                distribution: LuniDistribution::FlexStart,
                padding: LuniRect {
                    left: 0,
                    bottom: 4,
                    right: 0,
                    top: 4,
                },
                gap: 0,
                template: LuniChildTemplate {
                    align: LuniAlign::FlexStart,
                    margin: LuniRect {
                        left: 4,
                        bottom: 4,
                        right: 4,
                        top: 4,
                    },
                    basis: 40,
                    max: None,
                    min: None,
                    grow: 0.0,
                    shrink: 0.0,
                    cross: 40,
                },
            },
        );

        for (i, entry) in self.tabs.iter().enumerate() {
            let button_rect = world.build(luni.child(LuniChild::default()));
            let active = self.active;
            let button_selected = ValBind::new(world, move |world| world.fetch(active).val == i)
                .listen(active)
                .finish();
            let button = world.insert(Button {
                common: self.common,
                rect: button_rect,
                theme: ButtonTheme {
                    color: theme.color.surface_container,
                    on_color: theme.color.on_surface,
                    active_color: theme.color.primary_container,
                    active_on_color: theme.color.on_primary_container,
                    ..ButtonTheme::surface(&theme)
                },
                selected: button_selected,
                hovering: world.insert(Val::new(false)),
            });
            world.insert(ButtonImage {
                common: self.common,
                source: button,
                image: entry.icon,
                transform: TransformValue::center(12),
            });

            let indicator_rect = world.build(Transform::new(
                TransformValue::anchor((0.0, 0.5), IRect::new_half(IVec2::ZERO, UVec2::new(1, 16))),
                button_rect,
            ));
            world.insert(RRect {
                common: WidgetCommon {
                    visible: entry.visible,
                    ..self.common
                },
                rect: indicator_rect,
                order: 100,
                color: theme.color.primary,
                radius: 0.0,
                width: 0.0,
            });

            ValBind::attach(world, entry.rect, move |world| world.fetch(main_rect).val)
                .listen(main_rect)
                .finish();

            ValBind::attach(world, entry.visible, move |world| {
                let this = world.fetch(handle);
                world.fetch(button_selected).val && world.fetch(this.common.visible).val
            })
            .listen(button_selected)
            .listen(self.common.visible)
            .finish();

            world.observer(button, move |&ButtonClick, world| {
                let this = world.fetch(handle);
                let mut active = world.fetch_mut(this.active);
                active.val = i;
                world.queue_trigger(active.handle(), ValChanged);
            });
        }

        world.insert(luni);
    }
}

fn side_transform() -> TransformValue {
    TransformValue {
        left: TransformEdge {
            anchor: 0.0,
            offset: 0,
        },
        down: TransformEdge {
            anchor: 0.0,
            offset: 0,
        },
        right: TransformEdge {
            anchor: 0.0,
            offset: 48,
        },
        up: TransformEdge {
            anchor: 1.0,
            offset: 0,
        },
    }
}

fn main_transform() -> TransformValue {
    TransformValue {
        left: TransformEdge {
            anchor: 0.0,
            offset: 48,
        },
        down: TransformEdge {
            anchor: 0.0,
            offset: 0,
        },
        right: TransformEdge {
            anchor: 1.0,
            offset: 0,
        },
        up: TransformEdge {
            anchor: 1.0,
            offset: 0,
        },
    }
}
