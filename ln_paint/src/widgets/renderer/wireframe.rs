use glam::{UVec2, Vec4};
use ln_world::{Element, Handle, World};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::{
    layout::val::{Val, ValChanged},
    lnwin::Lnwindow,
    measures::IRect,
    render::{MSAA_STATE, Render, RenderDrawControl, camera::CameraBind},
    widgets::{common::WidgetCommon, shaders::shader_compile},
};

pub struct Wireframe {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub order: isize,
    pub color: Handle<Val<Vec4>>,
    pub radius: f32,
    pub width: f32,
    /// 0.0 inside, 0.5 centered, 1.0 outside.
    pub align: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct WireframeRect {
    origin: [i32; 2],
    extend: [u32; 2],
    edge: UVec2,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct WireframeMaterial {
    color: Vec4,
    radius: f32,
    width: f32,
    align: f32,
    _pad: f32,
}

pub struct WireframePipeline {
    pipeline: RenderPipeline,
    bind: BindGroupLayout,
}

const EDGE_MARGIN: f32 = 20.0;

// TODO less over-draw with more quads

impl Wireframe {
    pub fn init(&self, world: &World, this: Handle<Self>) {
        world.scope("wireframe");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);
        world.dependency(this, self.color);

        assert!(self.width >= 0.0, "wireframe width must be non-negative");

        let rect = world.fetch(self.rect).val;
        let color = world.fetch(self.color).val;
        let render = world.single_fetch::<Render>();
        let pipeline = world.single_fetch::<WireframePipeline>();
        let device = &render.device;

        let rectangle_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("wireframe_rect"),
            contents: bytemuck::bytes_of(&WireframeRect {
                origin: rect.origin.into(),
                extend: rect.extend.into(),
                edge: self.edge(),
            }),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let material_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("wireframe"),
            contents: bytemuck::bytes_of(&self.material(color)),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind = device.create_bind_group(&BindGroupDescriptor {
            label: Some("wireframe"),
            layout: &pipeline.bind,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: rectangle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: material_buffer.as_entire_binding(),
                },
            ],
        });

        world.insert(RenderDrawControl {
            common: self.common,
            order: self.order,
            draw: Box::new(move |world, rpass, extra| {
                let pipeline = world.single_fetch::<WireframePipeline>();
                let camera = extra.camera;

                let (start, end) = extra.diagnosis.assign("main > wireframe");
                extra.diagnosis.write(rpass, start);

                rpass.set_pipeline(&pipeline.pipeline);
                rpass.set_bind_group(0, &camera.bind, &[]);
                rpass.set_bind_group(1, &bind, &[]);
                rpass.draw(0..4, 0..1);

                extra.diagnosis.write(rpass, end);
            }),
        });

        world.observer(self.rect, move |ValChanged, world| {
            let this = world.fetch(this);
            let rect = world.fetch(this.rect).val;
            let render = world.single_fetch::<Render>();
            let uniform = WireframeRect {
                origin: rect.origin.into(),
                extend: rect.extend.into(),
                edge: this.edge(),
            };

            render
                .queue
                .write_buffer(&rectangle_buffer, 0, bytemuck::bytes_of(&uniform));
            let lnwindow = world.single_fetch::<Lnwindow>();
            lnwindow.window.request_redraw();
        });

        world.observer(self.color, move |ValChanged, world| {
            let this = world.fetch(this);
            let color = world.fetch(this.color).val;
            let material = this.material(color);
            let render = world.single_fetch::<Render>();

            render
                .queue
                .write_buffer(&material_buffer, 0, bytemuck::bytes_of(&material));
            let lnwindow = world.single_fetch::<Lnwindow>();
            lnwindow.window.request_redraw();
        });
    }

    fn edge(&self) -> UVec2 {
        let outer = (self.width * self.align).max(0.0);
        UVec2::splat((outer + EDGE_MARGIN).ceil() as u32)
    }

    fn material(&self, color: Vec4) -> WireframeMaterial {
        WireframeMaterial {
            color,
            radius: self.radius,
            width: self.width,
            align: self.align,
            _pad: 0.0,
        }
    }
}

impl WireframePipeline {
    pub fn from_world(world: &World) -> Self {
        let render = world.single_fetch::<Render>();
        let camera = world.single_fetch::<CameraBind>();
        let device = &render.device;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("wireframe"),
            source: ShaderSource::Wgsl(shader_compile(include_str!("wireframe.wgsl"), &[]).into()),
        });

        let bind = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("wireframe"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("wireframe"),
            bind_group_layouts: &[Some(&camera.layout), Some(&bind)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("wireframe"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: render.target_format(),
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            depth_stencil: None,
            multisample: MSAA_STATE,
            multiview_mask: None,
            cache: render.cache.as_ref(),
        });

        WireframePipeline { pipeline, bind }
    }
}

impl Element for WireframePipeline {}

impl Element for Wireframe {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        self.init(world, this);
    }
}
