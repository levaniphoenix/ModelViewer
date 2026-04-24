use bytemuck::cast_slice;
use eframe::wgpu;
use eframe::wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, PipelineLayoutDescriptor,
    RenderPipeline, ShaderModule, ShaderModuleDescriptor, ShaderStages,
};
use eframe::wgpu::util::{BufferInitDescriptor, DeviceExt};
use glam::Vec3;
use crate::camera::{Camera, CameraUniform};
use crate::mesh::{build_grid, LineVertex, MeshPrimitive, Vertex};

pub struct GpuPrimitive {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

pub struct Resources {
    pub shader: ShaderModule,
    pub pipeline: RenderPipeline,
    pub sky_pipeline: RenderPipeline,
    pub grid_pipeline: RenderPipeline,
    pub grid_vertex_buffer: wgpu::Buffer,
    pub grid_vertex_count: u32,
    pub bone_pipeline: RenderPipeline,
    pub bone_vertex_buffer: Option<wgpu::Buffer>,
    pub bone_vertex_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_buffer_bind_group: wgpu::BindGroup,
    pub camera: Camera,
    pub primitives: Vec<GpuPrimitive>,
}

impl Resources {
    pub fn new(
        device: &wgpu::Device,
        target_format: ColorTargetState,
        mesh_primitives: &[MeshPrimitive],
        bones: &[LineVertex],
    ) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("scene renderer"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/scene.wgsl").into()),
        });

        let mut bb_min = Vec3::splat(f32::INFINITY);
        let mut bb_max = Vec3::splat(f32::NEG_INFINITY);
        for prim in mesh_primitives {
            for v in &prim.vertices {
                let p = Vec3::from(v.position);
                bb_min = bb_min.min(p);
                bb_max = bb_max.max(p);
            }
        }
        let target = if bb_min.is_finite() && bb_max.is_finite() {
            (bb_min + bb_max) * 0.5
        } else {
            Vec3::ZERO
        };

        let camera = Camera {
            position: target + Vec3::new(0.0, 1.0, 2.0),
            target,
            rotation: Default::default(),
            up: (0.0, 1.0, 0.0).into(),
            fov: 45.0,
            z_far: 1000.0,
            z_near: 0.1,
            aspect: 4.0 / 3.0,
        };

        let camera_uniform = CameraUniform::new(&camera);

        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let uniform_buffer_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("uniform_buffer_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_buffer_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("uniform_buffer_bind_group"),
            layout: &uniform_buffer_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[Some(&uniform_buffer_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene_renderer_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::buffer_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(target_format.clone())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // no culling for now so we see everything
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        let sky_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("sky shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sky.wgsl").into()),
        });

        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sky_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sky_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(target_format.clone())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        let grid_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("grid shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/grid.wgsl").into()),
        });

        let grid_vertices = build_grid(10);
        let grid_vertex_count = grid_vertices.len() as u32;
        let grid_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: cast_slice(&grid_vertices),
            usage: BufferUsages::VERTEX,
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &grid_shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::buffer_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &grid_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(target_format.clone())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        let bone_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("bone shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/bone.wgsl").into()),
        });

        let bone_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bone_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bone_shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::buffer_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bone_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(target_format)],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        let (bone_vertex_buffer, bone_vertex_count) = if bones.is_empty() {
            (None, 0u32)
        } else {
            (
                Some(device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("bone_vertex_buffer"),
                    contents: cast_slice(bones),
                    usage: BufferUsages::VERTEX,
                })),
                bones.len() as u32,
            )
        };

        let primitives: Vec<GpuPrimitive> = mesh_primitives.iter().map(|prim| {
            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: cast_slice(&prim.vertices),
                usage: BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: cast_slice(&prim.indices),
                usage: BufferUsages::INDEX,
            });
            GpuPrimitive {
                vertex_buffer,
                index_buffer,
                index_count: prim.indices.len() as u32,
            }
        }).collect();

        Self {
            shader,
            pipeline,
            sky_pipeline,
            grid_pipeline,
            grid_vertex_buffer,
            grid_vertex_count,
            bone_pipeline,
            bone_vertex_buffer,
            bone_vertex_count,
            uniform_buffer,
            uniform_buffer_bind_group,
            camera,
            primitives,
        }
    }
}
