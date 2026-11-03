use ln_world::{Element, Handle, World};

use crate::{
    layer::{
        brush::param::BrushParamKey,
        input::{LayerInput, LayerInputMode},
        wrapper::{BrushConfigurationChanged, LayerPage},
    },
    layout::{
        luni::{LuniAlign, LuniAxis, LuniChild, LuniChildTemplate, LuniFlex, LuniParent, LuniRect},
        packer::Packer,
        transfer::Transfer,
        transform::{Transform, TransformEdge, TransformValue},
        val::{Val, ValChanged},
    },
    measures::{Axis, IRect},
    theme::ThemeVal,
    widgets::{
        common::WidgetCommon,
        components::button::{Button, ButtonClick, ButtonImage, ButtonToggle},
        panel::{Panel, PanelAttr, brush_panel::BrushPanel, color_picker::ColorPickerButton},
        renderer::svg::svg_render,
        slider::{Slider, SliderLabel, SliderValue},
    },
};

pub struct SideDocker {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for SideDocker {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("side_docker");
        world.set_autodeps(Some(this));

        let theme = world.single_fetch::<ThemeVal>();
        let layer = world.single::<LayerPage>();

        let panel_rect = world.build(Transform::new(
            TransformValue {
                left: TransformEdge {
                    anchor: 0.0,
                    offset: 24,
                },
                down: TransformEdge {
                    anchor: 0.5,
                    offset: -280,
                },
                right: TransformEdge {
                    anchor: 0.0,
                    offset: 24 + 44,
                },
                up: TransformEdge {
                    anchor: 0.5,
                    offset: 280,
                },
            },
            self.rect,
        ));

        world.insert(Panel {
            common: self.common,
            rect: panel_rect,
            attr: PanelAttr {
                shadow: true,
                round: true,
            },
        });

        let mut luni = LuniFlex::new(
            panel_rect,
            LuniParent {
                axis: LuniAxis::Column,
                template: LuniChildTemplate {
                    basis: 36,
                    cross: 36,
                    align: LuniAlign::Center,
                    ..Default::default()
                },
                padding: LuniRect {
                    left: 0,
                    bottom: 4,
                    right: 0,
                    top: 4,
                },
                gap: 4,
                ..Default::default()
            },
        );

        let eraser_selected = Packer::derive(world, move |world| {
            matches!(
                world
                    .single_fetch::<LayerPage>()
                    .active()
                    .toggle(BrushParamKey::Erase),
                Some(true)
            )
        })
        .observer(layer, move |&BrushConfigurationChanged, world| {
            matches!(
                world
                    .single_fetch::<LayerPage>()
                    .active()
                    .toggle(BrushParamKey::Erase),
                Some(true)
            )
        })
        .finish();

        let docker_button =
            |luni: &mut LuniFlex, image_bytes: &[u8], selected: Handle<Val<bool>>| {
                let rect = world.build(luni.child(LuniChild::default()));
                world.insert(Button {
                    common: self.common,
                    rect,
                    theme: theme.button(),
                    image: Some(ButtonImage::new(svg_render(image_bytes), theme.distinct, 8)),
                    selected,
                    hovering: world.insert(Val::new(false)),
                })
            };

        let brush_visible = world.insert(Val::new(false));
        let brush_menu_rect = world.build(luni.child(LuniChild::default()));
        let brush_menu = world.insert(Button {
            common: self.common,
            rect: brush_menu_rect,
            theme: theme.button(),
            image: Some(ButtonImage::new(
                svg_render(include_bytes!("../../../res/interface/pen.svg")),
                theme.distinct,
                8,
            )),
            selected: brush_visible,
            hovering: world.insert(Val::new(false)),
        });
        world.insert(ButtonToggle { source: brush_menu });

        let eraser = docker_button(
            &mut luni,
            include_bytes!("../../../res/interface/eraser.svg"),
            eraser_selected,
        );

        let color_visible = world.insert(Val::new(false));
        let color_picker_rect = world.build(luni.child(LuniChild::default()));
        world.insert(ColorPickerButton {
            common: self.common,
            rect: color_picker_rect,
            selected: color_visible,
        });

        let selection = docker_button(
            &mut luni,
            include_bytes!("../../../res/interface/square-dashed.svg"),
            world.insert(Val::new(false)),
        );

        let undo = docker_button(
            &mut luni,
            include_bytes!("../../../res/interface/undo-2.svg"),
            world.insert(Val::new(false)),
        );

        let redo = docker_button(
            &mut luni,
            include_bytes!("../../../res/interface/redo-2.svg"),
            world.insert(Val::new(false)),
        );

        world.build(luni.child(LuniChild {
            basis: Some(0),
            grow: Some(1.0),
            ..Default::default()
        }));

        let slider_rect = world.build(luni.child(LuniChild {
            basis: Some(160),
            shrink: Some(1.0),
            margin: Some(LuniRect {
                left: 0,
                bottom: 5,
                right: 0,
                top: 5,
            }),
            ..Default::default()
        }));

        let touch = docker_button(
            &mut luni,
            include_bytes!("../../../res/interface/pointer.svg"),
            world.insert(Val::new(false)),
        );

        let pipette = docker_button(
            &mut luni,
            include_bytes!("../../../res/interface/pipette.svg"),
            world.insert(Val::new(false)),
        );

        let brush_panel_rect = world.build(Transform::anchor(
            (1.0, 1.0),
            IRect::new_extend(20, -480, 364, 480),
            brush_menu_rect,
        ));

        world.insert(BrushPanel {
            common: WidgetCommon {
                camera: self.common.camera,
                visible: brush_visible,
            },
            rect: brush_panel_rect,
        });

        let value = world.insert(Val::new(0.67));
        let slider = world.insert(Slider {
            common: self.common,
            rect: slider_rect,
            axis: Axis::Up,
            value,
            pressed: false,
        });

        let slider_label_text = Transfer::derive(world, value).build(|&value| {
            let scale = (value.exp2() - 1.0) * 127.5 + 0.5;
            format!("{scale:.2} px")
        });
        world.insert(SliderLabel {
            text: slider_label_text,
            clockwise: true,
            source: slider,
            hover: false,
            common: self.common,
        });

        world.observer(brush_visible, move |&ValChanged, world| {
            if world.fetch(brush_visible).val {
                world.fetch_mut(color_visible).val = false;
                world.queue_trigger(color_visible, ValChanged);
            }
        });

        world.observer(color_visible, move |&ValChanged, world| {
            if world.fetch(color_visible).val {
                world.fetch_mut(brush_visible).val = false;
                world.queue_trigger(brush_visible, ValChanged);
            }
        });

        world.observer(eraser, move |&ButtonClick, world| {
            let mut layer = world.fetch_mut(layer);
            let is_erase = matches!(layer.active().toggle(BrushParamKey::Erase), Some(true));
            layer
                .active_mut()
                .set_toggle(BrushParamKey::Erase, !is_erase);
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        world.insert(ButtonToggle { source: pipette });
        world.observer(pipette, move |&ButtonClick, world| {
            let mut input = world.single_fetch_mut::<LayerInput>();
            input.hold_pick = !input.hold_pick;
        });

        world.insert(ButtonToggle { source: selection });
        // TODO selection state is not synced with LayerInput mode
        world.observer(selection, move |&ButtonClick, world| {
            let mut input = world.single_fetch_mut::<LayerInput>();
            input.mode = LayerInputMode::Area;
        });

        world.insert(ButtonToggle { source: touch });
        world.observer(touch, move |&ButtonClick, world| {
            let mut input = world.single_fetch_mut::<LayerInput>();
            input.touch_draw = !input.touch_draw;
        });

        world.observer(undo, move |&ButtonClick, world| {
            let mut layer = world.fetch_mut::<LayerPage>(layer);
            layer.undo();
        });

        world.observer(redo, move |&ButtonClick, world| {
            let mut layer = world.fetch_mut::<LayerPage>(layer);
            layer.redo();
        });

        world.observer(slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            let scale = (value.exp2() - 1.) * 127.5 + 0.5;
            if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::Size) {
                param.scale = scale;
            }

            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        world.observer(layer, move |&BrushConfigurationChanged, world| {
            let value = {
                let layer = world.fetch(layer);
                layer
                    .active()
                    .scalar(BrushParamKey::Size)
                    .map(|param| param.scale)
                    .unwrap_or(0.0)
            };
            set_slider_value(world, slider, ((value - 0.5) / 127.5 + 1.).log2());
        });

        world.insert(luni);

        world.dependency(this, self.rect);
    }
}

fn set_slider_value(world: &World, slider: Handle<Slider>, next: f32) {
    let handle = world.fetch(slider).value;
    let mut value = world.fetch_mut(handle);
    if value.val != next {
        value.val = next;
        drop(value);
        world.queue_trigger(handle, ValChanged);
    }
}
