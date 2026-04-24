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
use crate::mesh::{build_grid, LineVertex, Material, MeshPrimitive, Vertex};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub base_color: [f32; 4],
}

pub struct GpuMaterial {
    pub buffer: wgpu::Buffer,
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl GpuMaterial {
    pub fn rebuild_bind_group(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) {
        self.bind_group = device.create_bind_group(&BindGroupDescriptor {
            label,
            layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: self.buffer.as_entire_binding() },
                BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.texture_view) },
                BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
    }
}

pub struct GpuPrimitive {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub material: Option<usize>,
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
    pub materials: Vec<GpuMaterial>,
    pub default_material: GpuMaterial,
    pub material_bind_group_layout: wgpu::BindGroupLayout,
}

impl Resources {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: ColorTargetState,
        mesh_primitives: &[MeshPrimitive],
        bones: &[LineVertex],
        materials: &[Material],
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

        let material_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("material_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
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
                    ty: BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let white_pixel: [u8; 4] = [255, 255, 255, 255];
        let make_gpu_material = |label: &str, base_color: [f32; 4]| -> GpuMaterial {
            let buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some(label),
                contents: cast_slice(&[MaterialUniform { base_color }]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                texture.as_image_copy(),
                &white_pixel,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            );
            let texture_view = texture.create_view(&Default::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            });
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some(label),
                layout: &material_bind_group_layout,
                entries: &[
                    BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                    BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&texture_view) },
                    BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });
            GpuMaterial { buffer, texture, texture_view, sampler, bind_group }
        };

        let default_material = make_gpu_material("default_material", [0.8, 0.8, 0.8, 1.0]);
        let gpu_materials: Vec<GpuMaterial> = materials.iter().enumerate()
            .map(|(i, m)| make_gpu_material(&format!("material_{i}"), m.base_color))
            .collect();

        let scene_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("scene_pipeline_layout"),
            bind_group_layouts: &[
                Some(&uniform_buffer_bind_group_layout),
                Some(&material_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[Some(&uniform_buffer_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene_renderer_pipeline"),
            layout: Some(&scene_pipeline_layout),
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
                material: prim.material,
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
            materials: gpu_materials,
            default_material,
            material_bind_group_layout,
        }
    }
}
