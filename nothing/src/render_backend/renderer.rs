use crate::types;
use std::borrow::Cow;
// First number is the size of Rectangle struct (with padding).
// Second is in this case maximum number of allowed elements (can easily go into
// high thousands).
const RECTANGLE_BUFFER_SIZE: i32 = 16 * 1024;
const SAMPLE_COUNT: i32 = 4;

pub struct Renderer {
    pub data: [f32; RECTANGLE_BUFFER_SIZE as usize],
    pub count: i32,
    pub v_buffer: wgpu::Buffer,
    pub r_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        color_textture_view: wgpu::TextureView,
        surface: wgpu::Surface,
        adapter: wgpu::Adapter,
    ) -> Self {
        let node_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/node.wgsl"))),
        });

        let v_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex"),
            // 2 triangle, 2 (x, y), 3 points, 4 bytes per float
            size: 2 * 2 * 3 * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let r_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rectangle"),
            // 4 bytes per float
            size: (RECTANGLE_BUFFER_SIZE * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bindgrouplayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rectangle bindgroup layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipelinelayout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rectangle pipeline layout"),
            bind_group_layouts: &[&bindgrouplayout],
            push_constant_ranges: &[],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rectangle bindgroup"),
            layout: &bindgrouplayout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: r_buffer.as_entire_binding(),
            }],
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipelinelayout),
            vertex: wgpu::VertexState {
                module: &node_shader_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &node_shader_module,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Just regular full-screen quad consisting of two triangles.
        const VERTICIES: [i32; 12] = [0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1];

        queue.write_buffer(&v_buffer, 0, bytemuck::cast_slice(&VERTICIES));

        return Self {
            data: [0.0; RECTANGLE_BUFFER_SIZE as usize],
            count: 0,
            v_buffer,
            r_buffer,
            bind_group,
            pipeline,
        };
    }

    pub fn rectangle(
        &mut self,
        color: [f32; 4],
        position: [f32; 2],
        size: [f32; 2],
        corners: [f32; 4],
        sigma: f32,
        width: f32,
        height: f32,
    ) {
        //
    }
}
