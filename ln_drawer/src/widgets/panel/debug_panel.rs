use cosmic_text::{Attrs, Family, Metrics};
use glam::I64Vec2;
use ln_world::{Element, Handle, World};

use crate::{
    layer::wrapper::{LayerDebugMessage, LayerPage},
    layout::{
        transform::{Transform, TransformValue},
        val::{Val, ValChanged},
    },
    measures::IRect,
    render::Render,
    save::Save,
    theme::ThemeVal,
    widgets::{
        common::WidgetCommon,
        components::button::{Button, ButtonClick, ButtonImage, ButtonToggle},
        container::{
            ContainerAttr, ContainerDeco, ContainerDecoDropdown, ContainerEcho, ContainerWithCamera,
        },
        renderer::{
            svg::svg_render,
            text::{Text, TextAttr},
        },
    },
};

pub struct DebugPanel {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for DebugPanel {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("debug_panel");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.single_fetch::<ThemeVal>();

        let attr = ContainerAttr::default();
        let (container, common) = world.build(ContainerWithCamera {
            common: self.common,
            rect: self.rect,
            attr,
        });
        let restriction = world.build(ContainerDecoDropdown {
            height: None,
            outer: self.rect,
        });
        world.insert(ContainerDeco {
            container,
            restriction,
        });
        let local = world.build(ContainerEcho { container });

        let text_rect = world.build(Transform::new(TransformValue::shrink(24, 24), local));
        let debug_text = world.insert(Text {
            text: world.insert(Val::new("Hi there".to_string())),
            rect: text_rect,
            attr: TextAttr {
                metrics: Metrics::new(12.0, 18.0),
                attrs: Attrs::new().family(Family::Monospace),
                ..Default::default()
            },
            order: 50,
            common,
            modulate: theme.distinct,
            outdated: false,
        });

        world.observer(world.single::<Render>(), move |msg: &String, world| {
            let text = world.fetch(debug_text).text;
            let mut text = world.fetch_mut(text);
            if text.val != *msg {
                text.val.clone_from(msg);
                world.queue_trigger(text.handle(), ValChanged);
            }
        });

        world.observer(
            world.single::<LayerPage>(),
            move |LayerDebugMessage(msg), world| {
                let render = world.single_fetch::<Render>();
                if render.timestamp_poll {
                    return;
                }

                let text = world.fetch(debug_text).text;
                let mut text = world.fetch_mut(text);
                if text.val != *msg {
                    text.val.clone_from(msg);
                    world.queue_trigger(text.handle(), ValChanged);
                }
            },
        );

        let docker_button = |rect: Handle<Val<IRect>>, image_bytes: &[u8]| {
            world.insert(Button {
                common: WidgetCommon {
                    camera: common.camera,
                    visible: self.common.visible,
                },
                rect,
                theme: theme.button(),
                image: Some(ButtonImage::new(svg_render(image_bytes), theme.distinct, 8)),
                selected: world.insert(Val::new(false)),
                hovering: world.insert(Val::new(false)),
            })
        };

        let compass_rect = world.build(Transform::new(
            TransformValue::anchor((0.0, 0.0), IRect::new(20, 20, 40, 40)),
            local,
        ));
        let compass = docker_button(
            compass_rect,
            include_bytes!("../../../res/interface/compass.svg"),
        );
        world.observer(compass, move |&ButtonClick, world| {
            let container = world.single_fetch::<LayerPage>().container;
            let utils = world.single_fetch::<LayerPage>().camera_utils;
            let mut camera = world.fetch_mut(utils);
            camera.force_clear();
            camera.force_camera_center(I64Vec2::ZERO);
            camera.force_camera_zoom(0);
            camera.write_camera(world, container);
        });

        let page_debug_rect = world.build(Transform::new(
            TransformValue::anchor((0.33, 0.0), IRect::new(-10, 20, 10, 40)),
            local,
        ));
        let page_debug = docker_button(
            page_debug_rect,
            include_bytes!("../../../res/interface/bug.svg"),
        );
        world.insert(ButtonToggle { source: page_debug });
        world.observer(page_debug, move |&ButtonClick, world| {
            let mut layer = world.single_fetch_mut::<LayerPage>();
            layer.debug = !layer.debug;
        });

        let render_profile_rect = world.build(Transform::new(
            TransformValue::anchor((0.67, 0.0), IRect::new(-10, 20, 10, 40)),
            local,
        ));
        let render_profile = docker_button(
            render_profile_rect,
            include_bytes!("../../../res/interface/timer.svg"),
        );
        world.insert(ButtonToggle {
            source: render_profile,
        });
        world.observer(render_profile, move |&ButtonClick, world| {
            let mut render = world.single_fetch_mut::<Render>();
            render.timestamp_poll = !render.timestamp_poll;
        });

        let compact_rect = world.build(Transform::new(
            TransformValue::anchor((1.0, 0.0), IRect::new(-40, 20, -20, 40)),
            local,
        ));
        let compact = docker_button(
            compact_rect,
            include_bytes!("../../../res/interface/database-zap.svg"),
        );
        world.observer(compact, move |&ButtonClick, world| {
            log::debug!("on next startup database will be compacted");
            let save = world.single_fetch::<Save>();
            let mut sn_doc = save.sn_read();
            sn_doc.set_compact_on_startup(true);
            save.sn_write(&sn_doc);
        });
    }
}
