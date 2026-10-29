use cosmic_text::{Attrs, Metrics, Weight};
use ln_world::{Element, Handle, World};

use crate::{
    i18n::tr,
    layer::{
        brush::param::{BrushParamKey, BrushValue, BrushValueMut},
        wrapper::{BrushConfigurationChanged, LayerPage},
    },
    layout::{
        luni::{
            LuniAxis, LuniChild, LuniChildTemplate, LuniDistribution, LuniFlex, LuniParent,
            LuniRect,
        },
        transfer::Transfer,
        transform::{Transform, TransformEdge, TransformValue},
        val::{Val, ValChanged},
    },
    measures::{Axis, IRect},
    render::camera::Camera,
    theme::ThemeVal,
    widgets::{
        brush_preview::BrushPreviewDescriptor,
        common::WidgetCommon,
        container::{
            ContainerAttr, ContainerDeco, ContainerDecoDropdown, ContainerEcho, ContainerWithCamera,
        },
        renderer::text::{Text, TextAttr},
        slider::{Slider, SliderLabel, SliderValue},
    },
};

pub struct SettingPanel {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for SettingPanel {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("setting_panel");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let page = world.single_fetch::<LayerPage>();
        let theme = world.single_fetch::<ThemeVal>();

        let attr = ContainerAttr::default();
        let (container, common) = world.build(ContainerWithCamera {
            common: self.common,
            rect: self.rect,
            attr,
        });
        let local = world.build(ContainerEcho { container });

        let mut luni = LuniFlex::new(
            local,
            LuniParent {
                axis: LuniAxis::Column,
                distribution: LuniDistribution::FlexStart,
                padding: LuniRect::default(),
                gap: 4,
                template: LuniChildTemplate::default(),
            },
        );

        let preview_rect = world.build(luni.child(LuniChild {
            basis: Some(110),
            ..Default::default()
        }));

        let preview = world.insert(world.build(BrushPreviewDescriptor {
            common,
            rect: preview_rect,
            brush: page.active().dup(),
            background: theme.gray0,
            c0: theme.c0,
            c1: theme.c1,
            bc: theme.gray4,
        }));
        let settings_rect = world.build(luni.child(LuniChild {
            basis: Some(200),
            shrink: Some(1.0),
            grow: Some(1.0),
            ..Default::default()
        }));
        world.insert(TextSettingPanel {
            common: WidgetCommon {
                camera: common.camera,
                visible: self.common.visible,
            },
            rect: settings_rect,
        });

        world.insert(luni);

        world.observer(page.handle(), move |&BrushConfigurationChanged, world| {
            let mut preview = world.fetch_mut(preview);
            let layer_instance = world.single_fetch::<LayerPage>();
            preview.outdated = true;
            preview.brush = layer_instance.active().dup();
        });
    }
}

pub struct TextSettingPanel {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for TextSettingPanel {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("text_setting_panel");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.single_fetch::<ThemeVal>();
        let layer = world.single::<LayerPage>();

        let attr = ContainerAttr::default();
        let (container, common) = world.build(ContainerWithCamera {
            common: self.common,
            rect: self.rect,
            attr,
        });
        let restriction = world.build(ContainerDecoDropdown {
            height: Some((108 + 4) * 8 + 48 + 8),
            outer: self.rect,
        });
        world.insert(ContainerDeco {
            container,
            restriction,
        });
        let local = world.build(ContainerEcho { container });

        let mut luni = LuniFlex::new(
            local,
            LuniParent {
                axis: LuniAxis::Column,
                distribution: LuniDistribution::FlexStart,
                padding: LuniRect {
                    left: 12,
                    bottom: 4,
                    right: 12,
                    top: 4,
                },
                gap: 4,
                template: LuniChildTemplate::default(),
            },
        );

        let label1_frame = world.build(luni.child(LuniChild {
            basis: Some(48),
            ..Default::default()
        }));
        let label1_rect = world.build(Transform::new(
            TransformValue::anchor((0.0, 0.0), IRect::new_extend(16, 0, 56, 18)),
            label1_frame,
        ));
        world.insert(Text {
            text: world.insert(Val::new(tr("settings.brush.title").to_string())),
            rect: label1_rect,
            attr: TextAttr {
                metrics: Metrics {
                    font_size: 14.0,
                    line_height: 18.0,
                },
                attrs: Attrs::new().weight(Weight::BOLD),
                ..Default::default()
            },
            order: 100,
            common,
            modulate: theme.gray4,
            outdated: false,
        });

        let flow_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.brush.flow.label").into(),
            flow_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.brush.flow.desc").into(),
            flow_frame,
            common.camera,
            self.common.visible,
        );
        let flow_slider =
            option_slider(world, flow_frame, common.camera, self.common.visible, |v| {
                format!("{v:.2}")
            });
        world.observer(flow_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            match layer.active_mut().param_mut(BrushParamKey::Flow) {
                Some(BrushValueMut::Scalar(param)) => param.scale = value,
                Some(BrushValueMut::Vec4(flow)) => flow.w = value,
                _ => {}
            }
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        let sigma_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.brush.sigma.label").into(),
            sigma_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.brush.sigma.desc").into(),
            sigma_frame,
            common.camera,
            self.common.visible,
        );
        let sigma_slider = option_slider(
            world,
            sigma_frame,
            common.camera,
            self.common.visible,
            |v| format!("{:.2}", v * 3.0),
        );
        world.observer(sigma_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::Sigma) {
                param.scale = value * 3.0;
            }
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        let softness_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.brush.softness.label").into(),
            softness_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.brush.softness.desc").into(),
            softness_frame,
            common.camera,
            self.common.visible,
        );
        let softness_slider = option_slider(
            world,
            softness_frame,
            common.camera,
            self.common.visible,
            |v| format!("{v:.2}"),
        );
        world.observer(softness_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::Softness) {
                param.scale = 1. - value;
            }
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        let spacing_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.brush.spacing.label").into(),
            spacing_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.brush.spacing.desc").into(),
            spacing_frame,
            common.camera,
            self.common.visible,
        );
        let spacing_slider = option_slider(
            world,
            spacing_frame,
            common.camera,
            self.common.visible,
            |v| format!("{v:.2}"),
        );
        world.observer(spacing_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::Spacing) {
                param.scale = value;
            }
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        let color_ratio_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.brush.color_ratio.label").into(),
            color_ratio_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.brush.color_ratio.desc").into(),
            color_ratio_frame,
            common.camera,
            self.common.visible,
        );
        let color_ratio_slider = option_slider(
            world,
            color_ratio_frame,
            common.camera,
            self.common.visible,
            |v| format!("{v:.2}"),
        );
        world.observer(color_ratio_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::ColorRatio) {
                param.scale = value;
            }
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        let sample_radius_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.brush.sample_radius.label").into(),
            sample_radius_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.brush.sample_radius.desc").into(),
            sample_radius_frame,
            common.camera,
            self.common.visible,
        );
        let sample_radius_slider = option_slider(
            world,
            sample_radius_frame,
            common.camera,
            self.common.visible,
            |v| format!("{v:.2}"),
        );
        world.observer(sample_radius_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::SampleRadius) {
                param.scale = value;
            }
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        let sample_rate_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.brush.sample_rate.label").into(),
            sample_rate_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.brush.sample_rate.desc").into(),
            sample_rate_frame,
            common.camera,
            self.common.visible,
        );
        let sample_rate_slider = option_slider(
            world,
            sample_rate_frame,
            common.camera,
            self.common.visible,
            |v| format!("{v:.2}"),
        );
        world.observer(sample_rate_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            if let Some(param) = layer.active_mut().scalar_mut(BrushParamKey::SampleRate) {
                param.scale = value;
            }
            world.queue_trigger(layer.handle(), BrushConfigurationChanged);
        });

        let zoom_frame = world.build(luni.child(LuniChild {
            basis: Some(108),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.page.zoom.label").into(),
            zoom_frame,
            common.camera,
            self.common.visible,
        );
        option_desc(
            world,
            tr("settings.page.zoom.desc").into(),
            zoom_frame,
            common.camera,
            self.common.visible,
        );
        let zoom_slider =
            option_slider(world, zoom_frame, common.camera, self.common.visible, |v| {
                format!("{:+.2}", (v as f64 - 0.8) * 3.0)
            });
        world.observer(zoom_slider, move |&SliderValue(value), world| {
            let mut layer = world.fetch_mut(layer);
            layer.zoom_offset = (value as f64 - 0.8) * 3.;
            set_slider_value(world, zoom_slider, value);
        });

        world.observer(layer, move |&BrushConfigurationChanged, world| {
            let layer = world.fetch(layer);

            // Flow //
            let flow = match layer.active().param(BrushParamKey::Flow) {
                Some(BrushValue::Scalar(param)) => param.scale,
                Some(BrushValue::Vec4(flow)) => flow.w,
                _ => 0.0,
            };
            set_slider_value(world, flow_slider, flow);

            // Sigma //
            let sigma = layer
                .active()
                .scalar(BrushParamKey::Sigma)
                .map(|param| param.scale)
                .unwrap_or(0.0);
            set_slider_value(world, sigma_slider, sigma / 3.0);

            // Softness
            let softness = layer
                .active()
                .scalar(BrushParamKey::Softness)
                .map(|param| 1. - param.scale)
                .unwrap_or(1.0);
            set_slider_value(world, softness_slider, softness);

            // Spacing //
            let spacing = layer
                .active()
                .scalar(BrushParamKey::Spacing)
                .map(|param| param.scale)
                .unwrap_or(0.1);
            set_slider_value(world, spacing_slider, spacing);

            // Color Ratio //
            let color_ratio = layer
                .active()
                .scalar(BrushParamKey::ColorRatio)
                .map(|param| param.scale)
                .unwrap_or(0.0);
            set_slider_value(world, color_ratio_slider, color_ratio);

            // Sample Radius //
            let sample_radius = layer
                .active()
                .scalar(BrushParamKey::SampleRadius)
                .map(|param| param.scale)
                .unwrap_or(0.0);
            set_slider_value(world, sample_radius_slider, sample_radius);

            // Sample Rate //
            let sample_rate = layer
                .active()
                .scalar(BrushParamKey::SampleRate)
                .map(|param| param.scale)
                .unwrap_or(0.0);
            set_slider_value(world, sample_rate_slider, sample_rate);
        });

        world.insert(luni);
    }
}

fn option_label(
    world: &World,
    text: String,
    frame: Handle<Val<IRect>>,
    camera: Handle<Camera>,
    visible: Handle<Val<bool>>,
) {
    let theme = world.single_fetch::<ThemeVal>();

    let rect = world.build(Transform::new(
        TransformValue {
            left: TransformEdge {
                anchor: 0.0,
                offset: 56,
            },
            down: TransformEdge {
                anchor: 1.0,
                offset: -36,
            },
            right: TransformEdge {
                anchor: 1.0,
                offset: -72,
            },
            up: TransformEdge {
                anchor: 1.0,
                offset: -16,
            },
        },
        frame,
    ));

    let text = world.insert(Val::new(text));

    world.insert(Text {
        text,
        rect,
        attr: TextAttr {
            metrics: Metrics {
                font_size: 16.0,
                line_height: 20.0,
            },
            ..Default::default()
        },
        order: 100,
        common: WidgetCommon { camera, visible },
        modulate: theme.distinct,
        outdated: false,
    });
}

fn option_desc(
    world: &World,
    text: String,
    frame: Handle<Val<IRect>>,
    camera: Handle<Camera>,
    visible: Handle<Val<bool>>,
) {
    let theme = world.single_fetch::<ThemeVal>();

    let rect = world.build(Transform::new(
        TransformValue {
            left: TransformEdge {
                anchor: 0.0,
                offset: 56,
            },
            down: TransformEdge {
                anchor: 1.0,
                offset: -72,
            },
            right: TransformEdge {
                anchor: 1.0,
                offset: -72,
            },
            up: TransformEdge {
                anchor: 1.0,
                offset: -36,
            },
        },
        frame,
    ));

    let text = world.insert(Val::new(text));

    world.insert(Text {
        text,
        rect,
        attr: TextAttr {
            metrics: Metrics {
                font_size: 12.0,
                line_height: 14.0,
            },
            ..Default::default()
        },
        order: 100,
        common: WidgetCommon { camera, visible },
        modulate: theme.gray4,
        outdated: false,
    });
}

fn option_slider(
    world: &World,
    frame: Handle<Val<IRect>>,
    camera: Handle<Camera>,
    visible: Handle<Val<bool>>,
    format: impl Fn(f32) -> String + Send + 'static,
) -> Handle<Slider> {
    let rect = world.build(Transform::new(
        TransformValue {
            left: TransformEdge {
                anchor: 0.0,
                offset: 56,
            },
            down: TransformEdge {
                anchor: 1.0,
                offset: -108,
            },
            right: TransformEdge {
                anchor: 1.0,
                offset: -72,
            },
            up: TransformEdge {
                anchor: 1.0,
                offset: -72,
            },
        },
        frame,
    ));

    let value = world.insert(Val::new(0.5));
    let slider = world.insert(Slider {
        common: WidgetCommon { camera, visible },
        value,
        axis: Axis::Right,
        rect,
        pressed: false,
    });

    let text = Transfer::derive(world, value).build(move |&value| format(value));

    world.insert(SliderLabel {
        text,
        clockwise: false,
        source: slider,
        hover: false,
        common: WidgetCommon { camera, visible },
    });

    slider
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
