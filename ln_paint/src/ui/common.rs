use lnworld::Handle;

use crate::{render::camera::Camera, ui::theme::Theme, val::Val};

#[derive(Debug, Clone, Copy)]
pub struct WidgetCommon {
    pub camera: Handle<Camera>,
    pub visible: Handle<Val<bool>>,
    pub theme: Handle<Theme>,
}
