use glam::{IVec2, UVec2, Vec4};
use ln_world::{Element, Handle, HandleGeneric, World};

use crate::{
    input::collider::ToolRectCollider,
    ui::{
        common::WidgetCommon,
        renderer::{rrect::RRect, wireframe::Wireframe},
    },
    unit::IRect,
    val::{Val, ValBind},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaMode {
    None,
    Four,
    Eight,
}

pub struct Area {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub color: Handle<Val<Vec4>>,
    pub mode: Handle<Val<AreaMode>>,
}

const FRAME_WIDTH: f32 = 1.0;
const FRAME_ALIGN: f32 = 0.5;
const FRAME_ORDER: isize = 1000;
const HANDLE_ORDER: isize = 1001;
const HANDLE_COUNT: usize = 8;
const HANDLE_HALF: UVec2 = UVec2::splat(4);
const HANDLE_RADIUS: f32 = 2.0;

impl Area {
    pub fn init(&self, world: &World, this: Handle<Self>) {
        world.scope("area");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);
        world.dependency(this, self.color);
        world.dependency(this, self.mode);

        world.insert(Wireframe {
            common: self.common,
            rect: self.rect,
            order: FRAME_ORDER,
            color: self.color,
            radius: 0.0,
            width: FRAME_WIDTH,
            align: FRAME_ALIGN,
        });

        let rect_handle = self.rect;
        let mode_handle = self.mode;
        let visible_handle = self.common.visible;

        let handle_rects: [Handle<Val<IRect>>; HANDLE_COUNT] = std::array::from_fn(|index| {
            ValBind::new(world, move |world| {
                handle_at(world.fetch(rect_handle).val, index)
            })
            .listen(rect_handle)
            .finish()
        });
        let handle_visibles: [Handle<Val<bool>>; HANDLE_COUNT] = std::array::from_fn(|index| {
            ValBind::new(world, move |world| {
                let mode = world.fetch(mode_handle).val;
                let visible = world.fetch(visible_handle).val;
                visible && handle_visible(mode, index)
            })
            .listen(mode_handle)
            .listen(visible_handle)
            .finish()
        });

        for index in 0..HANDLE_COUNT {
            let common = WidgetCommon {
                camera: self.common.camera,
                visible: handle_visibles[index],
            };

            world.insert(RRect {
                common,
                rect: handle_rects[index],
                order: HANDLE_ORDER,
                color: self.color,
                radius: HANDLE_RADIUS,
                width: 0.0,
            });

            world.insert(ToolRectCollider {
                common,
                target: this.untyped(),
                rect: handle_rects[index],
                order: HANDLE_ORDER,
            });
        }
    }
}

fn handle_at(rect: IRect, index: usize) -> IRect {
    let center = match index {
        0 => rect.left_down(),
        1 => rect.right_down(),
        2 => rect.right_up(),
        3 => rect.left_up(),
        4 => IVec2::new(rect.horizontal_center(), rect.down()),
        5 => IVec2::new(rect.right(), rect.vertical_center()),
        6 => IVec2::new(rect.horizontal_center(), rect.up()),
        7 => IVec2::new(rect.left(), rect.vertical_center()),
        _ => unreachable!("area has exactly {HANDLE_COUNT} handles"),
    };

    IRect::new_half(center, HANDLE_HALF)
}

fn handle_visible(mode: AreaMode, index: usize) -> bool {
    match mode {
        AreaMode::None => false,
        AreaMode::Four => index < 4,
        AreaMode::Eight => true,
    }
}

impl Element for Area {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        self.init(world, this);
    }
}
