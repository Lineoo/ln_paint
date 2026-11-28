use lnworld:{Element, Handle, World};

use crate::{
    paint::wrapper::LayerPage,
    ui::{common::WidgetCommon, section::side_docker::SideDocker},
    unit::{Axis, IRect},
    val::Val,
};

pub struct MobileUi {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for MobileUi {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("mobile_ui");
        world.set_autodeps(Some(this));
        world.dependency(this, self.common.visible);

        world.insert(LayerPage::new(world, self.common, self.rect));
        world.insert(SideDocker {
            common: self.common,
            rect: self.rect,
            axis: world.insert(Val::new(Axis::Left)),
        });
    }
}
