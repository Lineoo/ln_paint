use cosmic_text::{Attrs, Metrics, Weight};
use lnworld::{Element, Handle, World};

use crate::{
    color::srgb::Srgb,
    paint::{
        brush::{
            BrushParams,
            param::{BrushParamKey, BrushValue, BrushValueMut},
        },
        wrapper::LayerPage,
    },
    render::nest::{NestAttr, NestBuilder},
    ui::{
        common::WidgetCommon,
        i18n::tr,
        layout::{
            luni::{
                LuniAxis, LuniChild, LuniChildTemplate, LuniDistribution, LuniFlex, LuniParent,
                LuniRect,
            },
            transform::{Transform, TransformEdge, TransformValue},
        },
        renderer::{
            brush_preview::BrushPreviewDescriptor,
            text::{Text, TextAttr},
        },
        widget::{
            container::Container,
            slider::{Slider, SliderLabel, SliderValue},
        },
    },
    unit::{Axis, IRect},
    val::{Val, ValBind, ValChanged},
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
        let theme = world.fetch(self.common.theme);

        let attr = NestAttr::default();
        let (_, inner, common) = world.build(NestBuilder {
            common: self.common,
            rect: self.rect,
            attr,
        });
        let mut luni = LuniFlex::new(
            inner,
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

        let brush_color = ValBind::new(world, move |world| {
            let page = world.single_fetch::<LayerPage>();
            let this = world.fetch(this);
            let theme = world.fetch(this.common.theme);
            let fallback = world.fetch(theme.color.on_surface).val;
            page.active()
                .color()
                .map(Srgb::to_linear)
                .unwrap_or(fallback)
        })
        .listen(page.handle())
        .finish();

        let preview = world.insert(world.build(BrushPreviewDescriptor {
            common,
            rect: preview_rect,
            brush: page.active().dup(),
            bg: theme.color.surface,
            bg_color1: theme.color.primary_container,
            bg_color2: theme.color.tertiary_container,
            brush_color,
        }));
        let settings_rect = world.build(luni.child(LuniChild {
            basis: Some(200),
            shrink: Some(1.0),
            grow: Some(1.0),
            ..Default::default()
        }));
        world.insert(TextSettingPanel {
            common,
            rect: settings_rect,
        });

        world.insert(luni);

        world.observer(page.handle(), move |&ValChanged, world| {
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

struct OptionRow {
    key: BrushParamKey,
    frame: Handle<Val<IRect>>,
    supported: Handle<Val<bool>>,
    shown: bool,
}

const OPTION_SPECS: [(BrushParamKey, &str, &str); 7] = [
    (
        BrushParamKey::Flow,
        "settings.brush.flow.label",
        "settings.brush.flow.desc",
    ),
    (
        BrushParamKey::Sigma,
        "settings.brush.sigma.label",
        "settings.brush.sigma.desc",
    ),
    (
        BrushParamKey::Softness,
        "settings.brush.softness.label",
        "settings.brush.softness.desc",
    ),
    (
        BrushParamKey::Spacing,
        "settings.brush.spacing.label",
        "settings.brush.spacing.desc",
    ),
    (
        BrushParamKey::ColorRatio,
        "settings.brush.color_ratio.label",
        "settings.brush.color_ratio.desc",
    ),
    (
        BrushParamKey::SampleRadius,
        "settings.brush.sample_radius.label",
        "settings.brush.sample_radius.desc",
    ),
    (
        BrushParamKey::SampleRate,
        "settings.brush.sample_rate.label",
        "settings.brush.sample_rate.desc",
    ),
];

const TITLE: i32 = 48;
const SLIDER: i32 = 108;

impl Element for TextSettingPanel {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("text_setting_panel");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.fetch(self.common.theme);
        let layer = world.single_fetch::<LayerPage>();
        let supported_keys = layer.active().list_params();
        let layer = layer.retrieve();
        let panel_visible = self.common.visible;

        let attr = NestAttr::default();
        let (nest, local, common) = world.build(NestBuilder {
            common: self.common,
            rect: self.rect,
            attr,
        });
        let shown = OPTION_SPECS
            .iter()
            .filter(|spec| supported_keys.contains(&spec.0))
            .count();
        let row_count = world.insert(Val::new(shown as i32));
        let restriction = ValBind::new(world, move |world| {
            let inner = world.fetch(local).val;
            let rows = world.fetch(row_count).val;
            IRect::new(
                inner.left(),
                inner.up() - (SLIDER + 4) * (rows + 1) - TITLE * 2 - 8,
                inner.right(),
                inner.up(),
            )
        })
        .listen(self.rect)
        .listen(row_count)
        .finish();
        world.insert(Container {
            nest,
            theme: self.common.theme,
            restriction,
        });

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

        let brush_title_frame = world.build(luni.child(LuniChild {
            basis: Some(TITLE),
            ..Default::default()
        }));
        let brush_title_rect = world.build(Transform::new(
            TransformValue::anchor((0.0, 0.0), IRect::new_extend(16, 0, 56, 18)),
            brush_title_frame,
        ));
        world.insert(Text {
            text: world.insert(Val::new(tr("settings.brush.title").to_string())),
            rect: brush_title_rect,
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
            modulate: theme.color.primary,
            outdated: false,
        });

        let mut options = Vec::with_capacity(OPTION_SPECS.len());
        let mut sliders = Vec::with_capacity(OPTION_SPECS.len());

        for &(key, label, desc) in &OPTION_SPECS {
            let supported = world.insert(Val::new(supported_keys.contains(&key)));
            let visible = option_visible(world, panel_visible, supported);
            let frame = world.build(luni.child(LuniChild {
                basis: Some(SLIDER),
                ..Default::default()
            }));
            let common = WidgetCommon { visible, ..common };
            option_label(world, tr(label).into(), frame, common);
            option_desc(world, tr(desc).into(), frame, common);
            let slider =
                option_slider(world, frame, common, move |value| format_option(key, value));
            world.observer(slider, move |&SliderValue(value), world| {
                let mut layer = world.fetch_mut(layer);
                apply_option(layer.active_mut(), key, value);
                world.queue_trigger(layer.handle(), ValChanged);
            });
            options.push(OptionRow {
                key,
                frame,
                supported,
                shown: supported_keys.contains(&key),
            });
            sliders.push((key, slider));
        }

        let page_title_frame = world.build(luni.child(LuniChild {
            basis: Some(TITLE),
            ..Default::default()
        }));
        let page_title_rect = world.build(Transform::new(
            TransformValue::anchor((0.0, 0.0), IRect::new_extend(16, 0, 56, 18)),
            page_title_frame,
        ));
        world.insert(Text {
            text: world.insert(Val::new(tr("settings.page.title").to_string())),
            rect: page_title_rect,
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
            modulate: theme.color.primary,
            outdated: false,
        });

        let zoom_frame = world.build(luni.child(LuniChild {
            basis: Some(SLIDER),
            ..Default::default()
        }));
        option_label(
            world,
            tr("settings.page.zoom.label").into(),
            zoom_frame,
            common,
        );
        option_desc(
            world,
            tr("settings.page.zoom.desc").into(),
            zoom_frame,
            common,
        );
        let zoom_slider = option_slider(world, zoom_frame, common, |v| {
            format!("{:+.2}", (v as f64 - 0.8) * 3.0)
        });
        world.observer(zoom_slider, move |&SliderValue(value), world| {
            let slider = world.fetch(zoom_slider);
            let mut layer = world.fetch_mut(layer);
            let mut slider_value = world.fetch_mut(slider.value);
            layer.zoom_offset = (value as f64 - 0.8) * 3.;
            slider_value.val = value;
            world.queue_trigger(slider_value.handle(), ValChanged);
        });

        luni.children = option_children(&options, brush_title_frame, page_title_frame, zoom_frame);
        let luni_handle = world.insert(luni);

        world.observer(layer, move |&ValChanged, world| {
            let layer = world.fetch(layer);

            for &(key, slider) in &sliders {
                let slider = world.fetch(slider);
                let mut value = world.fetch_mut(slider.value);
                value.val = read_option(layer.active(), key);
                world.queue_trigger(value.handle(), ValChanged);
            }

            let supported_keys = layer.active().list_params();
            let mut changed = false;
            let mut shown = 0;
            for option in &mut options {
                let now = supported_keys.contains(&option.key);
                option.shown = now;
                let mut flag = world.fetch_mut(option.supported);
                if flag.val != now {
                    flag.val = now;
                    changed = true;
                    drop(flag);
                    world.queue_trigger(option.supported, ValChanged);
                }
                if now {
                    shown += 1;
                }
            }

            if changed {
                let mut luni = world.fetch_mut(luni_handle);
                luni.children =
                    option_children(&options, brush_title_frame, page_title_frame, zoom_frame);
                drop(luni);
                let mut rows = world.fetch_mut(row_count);
                rows.val = shown;
                drop(rows);
                world.queue_trigger(row_count, ValChanged);
                world.queue_trigger(local, ValChanged);
            }
        });
    }
}

fn option_children(
    options: &[OptionRow],
    brush_frame: Handle<Val<IRect>>,
    page_frame: Handle<Val<IRect>>,
    zoom_frame: Handle<Val<IRect>>,
) -> Vec<(Handle<Val<IRect>>, LuniChild)> {
    let mut children = Vec::with_capacity(options.len() + 2);
    children.push((
        brush_frame,
        LuniChild {
            basis: Some(TITLE),
            ..Default::default()
        },
    ));
    for option in options {
        if option.shown {
            children.push((
                option.frame,
                LuniChild {
                    basis: Some(SLIDER),
                    ..Default::default()
                },
            ));
        }
    }
    children.push((
        page_frame,
        LuniChild {
            basis: Some(TITLE),
            ..Default::default()
        },
    ));
    children.push((
        zoom_frame,
        LuniChild {
            basis: Some(SLIDER),
            ..Default::default()
        },
    ));
    children
}

fn option_visible(
    world: &World,
    panel: Handle<Val<bool>>,
    supported: Handle<Val<bool>>,
) -> Handle<Val<bool>> {
    ValBind::new(world, move |world| {
        world.fetch(panel).val && world.fetch(supported).val
    })
    .listen(panel)
    .listen(supported)
    .finish()
}

fn apply_option(brush: &mut dyn BrushParams, key: BrushParamKey, value: f32) {
    match key {
        BrushParamKey::Flow => match brush.param_mut(key) {
            Some(BrushValueMut::Scalar(param)) => param.scale = value,
            Some(BrushValueMut::Vec4(flow)) => flow.w = value,
            _ => {}
        },
        BrushParamKey::Sigma => {
            if let Some(param) = brush.scalar_mut(key) {
                param.scale = value * 3.0;
            }
        }
        BrushParamKey::Softness => {
            if let Some(param) = brush.scalar_mut(key) {
                param.scale = 1. - value;
            }
        }
        _ => {
            if let Some(param) = brush.scalar_mut(key) {
                param.scale = value;
            }
        }
    }
}

fn read_option(brush: &dyn BrushParams, key: BrushParamKey) -> f32 {
    match key {
        BrushParamKey::Flow => match brush.param(key) {
            Some(BrushValue::Scalar(param)) => param.scale,
            Some(BrushValue::Vec4(flow)) => flow.w,
            _ => 0.0,
        },
        BrushParamKey::Sigma => match brush.scalar(key) {
            Some(param) => param.scale / 3.0,
            None => 0.0,
        },
        BrushParamKey::Softness => match brush.scalar(key) {
            Some(param) => 1. - param.scale,
            None => 1.0,
        },
        BrushParamKey::Spacing => match brush.scalar(key) {
            Some(param) => param.scale,
            None => 0.1,
        },
        _ => match brush.scalar(key) {
            Some(param) => param.scale,
            None => 0.0,
        },
    }
}

fn format_option(key: BrushParamKey, value: f32) -> String {
    match key {
        BrushParamKey::Sigma => format!("{:.2}", value * 3.0),
        _ => format!("{value:.2}"),
    }
}

fn option_label(world: &World, text: String, frame: Handle<Val<IRect>>, common: WidgetCommon) {
    let theme = world.fetch(common.theme);

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
        common,
        modulate: theme.color.on_surface,
        outdated: false,
    });
}

fn option_desc(world: &World, text: String, frame: Handle<Val<IRect>>, common: WidgetCommon) {
    let theme = world.fetch(common.theme);

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
        common,
        modulate: theme.color.on_surface_variant,
        outdated: false,
    });
}

fn option_slider(
    world: &World,
    frame: Handle<Val<IRect>>,
    common: WidgetCommon,
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
                offset: -SLIDER,
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
        common,
        value,
        axis: Axis::Right,
        rect,
        hover: world.insert(Val::new(false)),
        pressed: world.insert(Val::new(false)),
    });

    let text = ValBind::transfer(world, value, move |&value| format(value));

    world.insert(SliderLabel {
        text,
        clockwise: false,
        source: slider,
        common,
    });

    slider
}
