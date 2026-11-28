use std::sync::Arc;

use cosmic_text::{Align, Attrs, Family, Metrics};
use lnpaint::{
    lnwin::Lnwin,
    ui::{
        common::WidgetCommon,
        layout::transform::{Transform, TransformValue},
        renderer::{
            rrect::RRect,
            text::{Text, TextAttr},
        },
    },
    unit::IRect,
    val::Val,
};
use lnworld::{Element, Handle, World};
use winit::event_loop::EventLoop;

/// Benchmark fonts: pinned explicitly so nothing is resolved by fallback.
const FONT_SANS: &str = "Source Han Sans CN";
const FONT_MONO: &str = "Noto Sans Mono";
const FONT_EMOJI: &str = "Noto Color Emoji";

fn main() {
    env_logger::init();

    let lnwin = Lnwin {
        payload: Some(Arc::new(|world, common, rect| {
            world.insert(TextRender { common, rect });
        })),
        ..Default::default()
    };

    let event_loop = EventLoop::builder().build().unwrap();
    event_loop.run_app(lnwin).unwrap();
}

pub struct TextRender {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
}

impl Element for TextRender {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("text_render");
        world.set_autodeps(Some(this));
        world.dependency(this, self.common.visible);

        let theme = world.fetch(self.common.theme);

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: -200,
            color: theme.color.surface,
            radius: 0.0,
            width: 0.0,
        });

        let row = |top: i32, height: i32| {
            world.build(Transform::anchor(
                (0.0, 1.0),
                IRect::new(40, -top - height, 760, -top),
                self.rect,
            ))
        };

        let title = row(30, 60);
        let latin = row(100, 84);
        let cjk = row(196, 90);
        let emoji = row(296, 84);
        let left = world.build(Transform::new(
            TransformValue::anchor((0.0, 1.0), IRect::new(40, -470, 260, -400)),
            self.rect,
        ));
        let center = world.build(Transform::new(
            TransformValue::anchor((0.0, 1.0), IRect::new(280, -470, 500, -400)),
            self.rect,
        ));
        let right = world.build(Transform::new(
            TransformValue::anchor((0.0, 1.0), IRect::new(520, -470, 760, -400)),
            self.rect,
        ));
        let small = row(490, 50);

        let text = |text: String, rect, metrics: Metrics, family: &'static str, modulate| Text {
            text: world.insert(Val::new(text)),
            rect,
            attr: TextAttr {
                metrics,
                attrs: Attrs::new().family(Family::Name(family)),
                align: Align::Left,
            },
            order: 10,
            common: self.common,
            modulate,
            outdated: false,
        };

        world.insert(text(
            "Text Render".into(),
            title,
            Metrics::new(32.0, 38.0),
            FONT_SANS,
            theme.color.primary,
        ));

        world.insert(text(
            "The quick brown fox jumps over the lazy dog.\nABCDEFGHIJKLMNOPQRSTUVWXYZ\n0123456789 !@#$%^&*()"
                .into(),
            latin,
            Metrics::new(18.0, 24.0),
            FONT_MONO,
            theme.color.on_surface,
        ));

        world.insert(text(
            "永和九年，岁在癸丑，暮春之初。\n中文字体测试：天地玄黄，宇宙洪荒。\n日本語のテキスト / 한국어 텍스트"
                .into(),
            cjk,
            Metrics::new(18.0, 26.0),
            FONT_SANS,
            theme.color.on_surface,
        ));

        world.insert(text(
            "😀 😁 🥰 😎 🤖 🍎 🌈 🚀 ❤️ 🔥 🟥 🟦\n⭐ 🌸 🐱 🍕 🎨 👀 🟩 🟨 🟪 🟧".into(),
            emoji,
            Metrics::new(24.0, 36.0),
            FONT_EMOJI,
            theme.color.on_surface,
        ));

        for (rect, align) in [
            (left, Align::Left),
            (center, Align::Center),
            (right, Align::Right),
        ] {
            world.insert(Text {
                text: world.insert(Val::new(format!("对齐 / {align:?}\n第二行 line two"))),
                rect,
                attr: TextAttr {
                    metrics: Metrics::new(14.0, 20.0),
                    attrs: Attrs::new().family(Family::Name(FONT_SANS)),
                    align,
                },
                order: 10,
                common: self.common,
                modulate: theme.color.primary,
                outdated: false,
            });
        }

        world.insert(text(
            "small text 12px：0123456789 中文小字".into(),
            small,
            Metrics::new(12.0, 18.0),
            FONT_SANS,
            theme.color.on_surface,
        ));
    }
}
