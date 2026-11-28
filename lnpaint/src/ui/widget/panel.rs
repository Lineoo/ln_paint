use glam::IVec2;
use lnworld::{Element, Handle, HandleGeneric, World};

use crate::{
    input::collider::ToolRectCollider,
    ui::{
        common::WidgetCommon,
        layout::transform::{Transform, TransformValue},
        renderer::rrect::RRect,
    },
    unit::IRect,
    val::Val,
};

pub struct PanelAttr {
    pub round: bool,
    pub shadow: bool,
}

pub struct Panel {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub attr: PanelAttr,
}

impl Panel {
    fn init(&self, world: &World, handle: Handle<Self>) {
        world.scope("panel");
        world.set_autodeps(Some(handle));
        world.dependency(handle, self.rect);
        world.dependency(handle, self.common.visible);

        let theme = world.fetch(self.common.theme);
        let shadow_offset = IVec2::ZERO;
        let shadow_blur = world.fetch(theme.shape.shadow_width).val;
        let roundness = match self.attr.round {
            true => world.fetch(theme.shape.corner_radius).val,
            false => 0.0,
        };

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: 0,
            color: theme.color.surface,
            radius: roundness,
            width: 0.0,
        });

        if self.attr.shadow {
            let transform = TransformValue::shift(shadow_offset.x, shadow_offset.y);
            let shadow_rect = world.build(Transform::new(transform, self.rect));
            world.insert(RRect {
                common: self.common,
                rect: shadow_rect,
                order: -1,
                color: theme.color.shadow,
                radius: roundness,
                width: shadow_blur,
            });
        }

        world.insert(ToolRectCollider {
            common: self.common,
            target: handle.untyped(),
            rect: self.rect,
            order: 0,
        });
    }
}

impl Element for Panel {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        self.init(world, this);
    }
}
