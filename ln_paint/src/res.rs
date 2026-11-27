#![cfg_attr(rustfmt, rustfmt::skip)]

pub struct Icon(());
impl Icon {
    pub const BRUSH:            &'static [u8] = include_bytes!("../res/icon/brush.svg"              );
    pub const BUG:              &'static [u8] = include_bytes!("../res/icon/bug.svg"                );
    pub const COMPASS:          &'static [u8] = include_bytes!("../res/icon/compass.svg"            );
    pub const DATABASE_ZAP:     &'static [u8] = include_bytes!("../res/icon/database-zap.svg"       );
    pub const DROPLET:          &'static [u8] = include_bytes!("../res/icon/droplet.svg"            );
    pub const ERASER:           &'static [u8] = include_bytes!("../res/icon/eraser.svg"             );
    pub const LAYERS:           &'static [u8] = include_bytes!("../res/icon/layers.svg"             );
    pub const PALETTE:          &'static [u8] = include_bytes!("../res/icon/palette.svg"            );
    pub const PENCIL_SPARKLES:  &'static [u8] = include_bytes!("../res/icon/pencil-sparkles.svg"    );
    pub const PENCIL:           &'static [u8] = include_bytes!("../res/icon/pencil.svg"             );
    pub const PEN:              &'static [u8] = include_bytes!("../res/icon/pen.svg"                );
    pub const PIPETTE:          &'static [u8] = include_bytes!("../res/icon/pipette.svg"            );
    pub const POINTER_OFF:      &'static [u8] = include_bytes!("../res/icon/pointer-off.svg"        );
    pub const POINTER:          &'static [u8] = include_bytes!("../res/icon/pointer.svg"            );
    pub const REDO_2:           &'static [u8] = include_bytes!("../res/icon/redo-2.svg"             );
    pub const SETTINGS:         &'static [u8] = include_bytes!("../res/icon/settings.svg"           );
    pub const SQUARE_DASHED:    &'static [u8] = include_bytes!("../res/icon/square-dashed.svg"      );
    pub const TIMER:            &'static [u8] = include_bytes!("../res/icon/timer.svg"              );
    pub const UNDO_2:           &'static [u8] = include_bytes!("../res/icon/undo-2.svg"             );
}

pub struct SourceHanSerifCN(());
impl SourceHanSerifCN {
    pub const FAMILY_NAME:      &'static str = "Source Han Serif CN";    
    pub const REGULAR:      &'static [u8] = include_bytes!("../fonts/SourceHanSerifCN-Regular.otf"  );
    pub const BOLD:         &'static [u8] = include_bytes!("../fonts/SourceHanSerifCN-Bold.otf"     );
}

pub struct SourceHanSansCN(());
impl SourceHanSansCN {
    pub const FAMILY_NAME:      &'static str = "Source Han Sans CN";    
    pub const REGULAR:      &'static [u8] = include_bytes!("../fonts/SourceHanSansCN-Regular.otf"   );
    pub const BOLD:         &'static [u8] = include_bytes!("../fonts/SourceHanSansCN-Bold.otf"      );
}