use nalgebra as na;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub struct Context {
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,

    pub size: winit::dpi::PhysicalSize<u32>,
}

impl Context {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let surface = wgpu::Surface::create(window);

        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: Default::default(),
            })
            .await;

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        Self {
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,

            size,
        }
    }

    pub async fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }
}

pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}
pub struct InterfacePass {
    pipeline: wgpu::RenderPipeline,
    uniforms_bind_group: wgpu::BindGroup,
    camera: na::Orthographic3<f32>,

    pub vertices: Vec<InterfaceVertex>,
    pub indices: Vec<u32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct InterfaceVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub index: u32,
}

unsafe impl bytemuck::Pod for InterfaceVertex {}
unsafe impl bytemuck::Zeroable for InterfaceVertex {}

impl Vertex for InterfaceVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;

        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct VertexUniforms {
    camera: na::Matrix4<f32>,
    transform: na::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for VertexUniforms {}
unsafe impl bytemuck::Zeroable for VertexUniforms {}

impl InterfacePass {
    fn new(ctx: &mut Context) -> Self {
        let vs_data = include_bytes!("shader/interface.vert.spv");
        let fs_data = include_bytes!("shader/interface.frag.spv");

        let vs_data = wgpu::read_spirv(std::io::Cursor::new(&vs_data[..])).unwrap();
        let fs_data = wgpu::read_spirv(std::io::Cursor::new(&fs_data[..])).unwrap();

        let vs_module = ctx.device.create_shader_module(&vs_data);
        let fs_module = ctx.device.create_shader_module(&fs_data);

        let camera = na::Orthographic3::new(0.0, 1.0, 0.0, 1.0, 10.0, 100.0);

        let uniforms = VertexUniforms {
            camera: camera.as_matrix().to_owned(),
            transform: na::Matrix4::identity(),
        };

        let uniforms = ctx.device.create_buffer_with_data(
            bytemuck::cast_slice(&[uniforms]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let uniforms_bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    bindings: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    }],
                });

        let uniforms_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uniforms_bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &uniforms,
                    range: 0..(std::mem::size_of::<VertexUniforms>() as wgpu::BufferAddress),
                },
            }],
        });

        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&uniforms_bind_group_layout],
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: &layout,
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::Back,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                }),
                color_states: &[wgpu::ColorStateDescriptor {
                    format: ctx.sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                depth_stencil_state: None,
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint32,
                    vertex_buffers: &[InterfaceVertex::desc()],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

        Self {
            pipeline,
            uniforms_bind_group,
            camera,
            vertices: vec![],
            indices: vec![],
        }
    }

    fn update(&mut self) {}

    fn render(&self, ctx: &mut Context) {
        let vertex_buffer = ctx.device.create_buffer_with_data(
            bytemuck::cast_slice(&self.vertices),
            wgpu::BufferUsage::VERTEX,
        );

        let index_buffer = ctx.device.create_buffer_with_data(
            bytemuck::cast_slice(&self.indices),
            wgpu::BufferUsage::INDEX,
        );

        let frame = ctx
            .swap_chain
            .get_next_texture()
            .expect("Timeout getting texture");

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Load,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    },
                }],
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            pass.set_vertex_buffer(0, &vertex_buffer, 0, 0);
            pass.set_index_buffer(&index_buffer, 0, 0);
            pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }

        ctx.queue.submit(&[encoder.finish()]);
    }
}

pub struct Application {
    interface_pass: InterfacePass,
}

impl Application {
    pub fn new(ctx: &mut Context) -> Self {
        let mut interface_pass = InterfacePass::new(ctx);

        interface_pass.vertices = vec![
            InterfaceVertex {
                pos: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                uv: [0.0, 0.0],
                index: 0,
            },
            InterfaceVertex {
                pos: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
                uv: [0.0, 0.0],
                index: 0,
            },
            InterfaceVertex {
                pos: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
                uv: [0.0, 0.0],
                index: 0,
            },
            InterfaceVertex {
                pos: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
                uv: [0.0, 0.0],
                index: 0,
            },
        ];

        interface_pass.indices = vec![0, 1, 3, 1, 2, 3];

        Self { interface_pass }
    }

    pub fn update(&mut self, ctx: &mut Context) {
        self.interface_pass.update();
    }

    pub fn render(&self, ctx: &mut Context) {
        self.interface_pass.render(ctx);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        true
    }
}

fn main() {
    use futures::executor::block_on;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Nomads of Myria")
        .build(&event_loop)
        .unwrap();

    let mut ctx = block_on(Context::new(&window));
    let mut app = Application::new(&mut ctx);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if app.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => *control_flow = ControlFlow::Exit,
                            _ => {}
                        },

                        WindowEvent::Resized(physical_size) => {
                            block_on(ctx.resize(*physical_size));
                        }

                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            block_on(ctx.resize(**new_inner_size));
                        }

                        _ => {}
                    }
                }
            }

            Event::RedrawRequested(_) => {
                app.update(&mut ctx);
                app.render(&mut ctx);
            }

            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
