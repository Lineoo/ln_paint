use glam::{IVec2, UVec2};
use ln_world::{Element, Handle, World};

use crate::{
    paint::{
        brush::param::BrushParamKey,
        input::{LayerDrag, LayerDraw, LayerInputMode},
        wrapper::LayerPage,
    },
    ui::{
        common::WidgetCommon,
        layout::{
            luni::{
                LuniAlign, LuniAxis, LuniChild, LuniChildTemplate, LuniFlex, LuniParent, LuniRect,
            },
            transform::{Transform, TransformValue},
        },
        renderer::svg::svg_render,
        section::{
            brush_panel::BrushPanel,
            color_picker::{ColorPickerButton, ColorPickerPanel},
        },
        widget::{
            button::{Button, ButtonClick, ButtonImage, ButtonTheme, ButtonToggle},
            panel::{Panel, PanelAttr},
            slider::{Slider, SliderLabel, SliderValue},
        },
    },
    unit::{Axis, IRect},
    val::{Val, ValBind, ValChanged},
};

pub struct SideDocker {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub axis: Handle<Val<Axis>>,
}

impl Element for SideDocker {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("side_docker");
        world.set_autodeps(Some(this));

        let theme = world.fetch(self.common.theme);
        let page = world.single::<LayerPage>();

        let axis = world.fetch(self.axis).val;
        let panel_rect = world.build(Transform::new(
            TransformValue::anchor(
                axis.anchor().into(),
                IRect::new_half(axis.direction() * -36, axis.half(22, 280)),
            ),
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

        let place_dir = axis.positive().rotate_cw();
        let mut luni = LuniFlex::new(
            panel_rect,
            LuniParent {
                axis: LuniAxis::from_axis(place_dir),
                template: LuniChildTemplate {
                    basis: 36,
                    cross: 36,
                    align: LuniAlign::Center,
                    ..Default::default()
                },
                padding: LuniRect {
                    left: 4 * place_dir.positive().horizontal(),
                    bottom: 4 * place_dir.positive().vertical(),
                    right: 4 * place_dir.positive().horizontal(),
                    top: 4 * place_dir.positive().vertical(),
                },
                gap: 4,
                ..Default::default()
            },
        );

        let docker_button = |luni: &mut LuniFlex, image_bytes: &[u8]| {
            let rect = world.build(luni.child(LuniChild::default()));
            Button {
                common: self.common,
                rect,
                theme: ButtonTheme::surface(&theme),
                image: Some(ButtonImage::new(svg_render(image_bytes), 8)),
                selected: world.insert(Val::new(false)),
                hovering: world.insert(Val::new(false)),
            }
        };

        let brush_visible = world.insert(Val::new(false));
        let color_visible = world.insert(Val::new(false));

        {
            let brush_menu_rect = world.build(luni.child(LuniChild::default()));
            let brush_menu = world.insert(Button {
                common: self.common,
                rect: brush_menu_rect,
                theme: ButtonTheme::surface(&theme),
                image: Some(ButtonImage::new(
                    svg_render(include_bytes!("../../../res/interface/pen.svg")),
                    8,
                )),
                selected: brush_visible,
                hovering: world.insert(Val::new(false)),
            });
            world.insert(ButtonToggle { source: brush_menu });

            let brush_panel_rect = world.build(Transform::anchor(
                (0.5, 0.5),
                IRect::new_half(
                    axis.flip().direction() * IVec2::new(182 + 34, 240 + 34),
                    UVec2::new(182, 240),
                ),
                panel_rect,
            ));

            world.insert(BrushPanel {
                common: WidgetCommon {
                    visible: brush_visible,
                    ..self.common
                },
                rect: brush_panel_rect,
            });
        }
        {
            let eraser_selected = ValBind::new(world, move |world| {
                let page = world.fetch(page);
                page.active()
                    .toggle(BrushParamKey::Erase)
                    .unwrap_or_default()
            })
            .listen(page)
            .finish();
            let eraser = world.insert(Button {
                common: self.common,
                rect: world.build(luni.child(LuniChild::default())),
                theme: ButtonTheme::surface(&theme),
                image: Some(ButtonImage::new(
                    svg_render(include_bytes!("../../../res/interface/eraser.svg")),
                    8,
                )),
                selected: eraser_selected,
                hovering: world.insert(Val::new(false)),
            });
            world.observer(eraser, move |&ButtonClick, world| {
                let mut page = world.fetch_mut(page);
                let is_erase = page
                    .active()
                    .toggle(BrushParamKey::Erase)
                    .unwrap_or_default();
                page.active_mut()
                    .set_toggle(BrushParamKey::Erase, !is_erase);
                world.queue_trigger(page.handle(), ValChanged);
            });
        }
        {
            let color_menu_rect = world.build(luni.child(LuniChild::default()));
            world.insert(ColorPickerButton {
                common: self.common,
                rect: color_menu_rect,
                selected: color_visible,
            });

            let color_panel_rect = world.build(Transform::anchor(
                (0.5, 0.5),
                IRect::new_half(
                    axis.flip().direction() * IVec2::new(182 + 34, 160 + 34),
                    UVec2::new(182, 160),
                ),
                panel_rect,
            ));

            world.insert(ColorPickerPanel {
                common: WidgetCommon {
                    visible: color_visible,
                    ..self.common
                },
                rect: color_panel_rect,
            });
        }
        {
            let rect = world.build(luni.child(LuniChild::default()));
            let input = world.single::<LayerDraw>();
            let selected = ValBind::new(world, |world| {
                let input = world.single_fetch::<LayerDraw>();
                matches!(input.mode, LayerInputMode::Area)
            })
            .listen(input)
            .finish();
            let selection = world.insert(Button {
                common: self.common,
                rect,
                theme: ButtonTheme::surface(&theme),
                image: Some(ButtonImage::new(
                    svg_render(include_bytes!("../../../res/interface/square-dashed.svg")),
                    8,
                )),
                selected,
                hovering: world.insert(Val::new(false)),
            });
            world.observer(selection, move |&ButtonClick, world| {
                let mut input = world.single_fetch_mut::<LayerDraw>();
                if let LayerInputMode::Area = input.mode {
                    input.mode = LayerInputMode::Normal;
                } else {
                    input.mode = LayerInputMode::Area;
                }
                world.queue_trigger(input.handle(), ValChanged);
            });
        }
        {
            let undo = world.insert(docker_button(
                &mut luni,
                include_bytes!("../../../res/interface/undo-2.svg"),
            ));
            world.observer(undo, move |&ButtonClick, world| {
                let mut layer = world.fetch_mut::<LayerPage>(page);
                layer.undo();
            });
        }
        {
            let redo = world.insert(docker_button(
                &mut luni,
                include_bytes!("../../../res/interface/redo-2.svg"),
            ));
            world.observer(redo, move |&ButtonClick, world| {
                let mut layer = world.fetch_mut::<LayerPage>(page);
                layer.redo();
            });
        }
        world.build(luni.child(LuniChild {
            basis: Some(0),
            grow: Some(1.0),
            ..Default::default()
        }));
        {
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
            let slider_value = ValBind::new(world, move |world| {
                let page = world.fetch(page);
                let curr = page
                    .active()
                    .scalar(BrushParamKey::Size)
                    .map(|param| param.scale)
                    .unwrap_or(0.0);
                ((curr - 0.5) / 127.5 + 1.).log2()
            })
            .listen(page)
            .finish();
            let slider = world.insert(Slider {
                common: self.common,
                rect: slider_rect,
                axis: place_dir.positive(),
                value: slider_value,
                hover: world.insert(Val::new(false)),
                pressed: false,
            });
            let slider_label_text = ValBind::transfer(world, slider_value, |&value| {
                let scale = (value.exp2() - 1.0) * 127.5 + 0.5;
                format!("{scale:.2} px")
            });
            world.insert(SliderLabel {
                text: slider_label_text,
                clockwise: true,
                source: slider,
                common: self.common,
            });
            world.observer(slider, move |&SliderValue(value), world| {
                let mut layer = world.fetch_mut(page);
                let scale = (value.exp2() - 1.) * 127.5 + 0.5;
                if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::Size) {
                    param.scale = scale;
                }
                world.queue_trigger(layer.handle(), ValChanged);
            });
        }
        {
            let touch = world.insert(docker_button(
                &mut luni,
                include_bytes!("../../../res/interface/pointer.svg"),
            ));
            world.insert(ButtonToggle { source: touch });
            world.observer(touch, move |&ButtonClick, world| {
                let mut input = world.single_fetch_mut::<LayerDrag>();
                input.touch_draw = !input.touch_draw;
            });
        }
        {
            let pipette = world.insert(docker_button(
                &mut luni,
                include_bytes!("../../../res/interface/pipette.svg"),
            ));
            world.insert(ButtonToggle { source: pipette });
            world.observer(pipette, move |&ButtonClick, world| {
                let mut input = world.single_fetch_mut::<LayerDraw>();
                input.hold_pick = !input.hold_pick;
            });
        }

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

        world.insert(luni);
    }
}
