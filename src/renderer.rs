use bytemuck::cast_slice;
use eframe::wgpu;
use eframe::wgpu::{BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BindingType, BufferBindingType, PipelineLayoutDescriptor, RenderPipeline, ShaderModule, ShaderModuleDescriptor, ShaderStages};
use eframe::wgpu::util::BufferInitDescriptor;
use eframe::wgpu::{BindGroupLayoutDescriptor, BufferUsages, ColorTargetState};
use eframe::wgpu::util::DeviceExt;
use eframe::wgpu::wgc::binding_model::CreateBindGroupError::DepthStencilAspect;
use crate::camera::{Camera, CameraUniform};

pub struct Resources{
    pub shader : ShaderModule,
    pub pipeline : RenderPipeline,
    pub uniform_buffer : wgpu::Buffer,
    pub uniform_buffer_bind_group : wgpu::BindGroup,
    pub  camera : Camera,
}

impl Resources{
    pub fn new(device : &wgpu::Device, target_format: ColorTargetState) -> Self{
        let shader  = device.create_shader_module(ShaderModuleDescriptor{
            label : Some("scene renderer"),
            source : wgpu::ShaderSource::Wgsl(include_str!("../shaders/scene.wgsl").into()),
        });
        
        let camera = Camera {
            position: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0 , 0.0).into(),
            rotation: Default::default(),
            up: (0.0, 1.0, 0.0).into(),
            fov: 45.0,
            z_far: 1000.0,
            z_near: 0.1,
            aspect: 4.0/3.0, //todo : get aspect ratio from window
        };
        
        let camera_uniform = CameraUniform::new(&camera);
        
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("uniform_buffer"),
            contents: cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        
        let uniform_buffer_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("uniform_buffer_bind_group_layout"),
            entries: &[BindGroupLayoutEntry{
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
       
        let uniform_buffer_bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("uniform_buffer_bind_group"),
            layout: &uniform_buffer_bind_group_layout,
            entries: &[BindGroupEntry{
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{ 
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
                buffers: &[], // vertex buffer layouts
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(target_format)],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });
        Self{shader,pipeline, uniform_buffer, uniform_buffer_bind_group, camera}
    }
}