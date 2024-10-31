use std::{
    io::{stdout, Write},
    ops::Range,
    time::Instant,
};

use wgpu::{include_wgsl, util::DeviceExt};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const VSYNC: bool = false;
pub const MAX_CURVE_COUNT: usize = 2;
const MAX_CURVE_VERTICES: usize = 1024;

const COLORS: [[f32; 3]; MAX_CURVE_COUNT] = [
    [1.0, 0.0, 0.0], // Red
    [0.0, 1.0, 0.0], // Green
    // [0.0, 0.0, 1.0], // Blue
    // [1.0, 1.0, 0.0], // Yellow
    // [1.0, 0.0, 1.0], // Magenta
    // [0.0, 1.0, 1.0], // Cyan
    // [0.5, 0.0, 0.0], // Dark Red
    // [0.0, 0.5, 0.0], // Dark Green
    // [0.0, 0.0, 0.5], // Dark Blue
    // [1.0, 0.5, 0.0], // Orange
    // [0.5, 0.0, 0.5], // Purple
    // [0.0, 0.5, 0.5], // Teal
    // [0.8, 0.4, 0.0], // Brown
    // [0.0, 0.4, 0.8], // Sky Blue
    // [0.4, 0.8, 0.0], // Lime Green
    // [0.8, 0.0, 0.4], // Pink
    // [0.6, 0.3, 0.3], // Light Brown
    // [0.3, 0.6, 0.3], // Light Green
    // [0.3, 0.3, 0.6], // Light Blue
    // [0.9, 0.7, 0.0], // Gold
    // [0.7, 0.0, 0.9], // Violet
    // [0.0, 0.7, 0.9], // Aquamarine
    // [0.9, 0.0, 0.7], // Hot Pink
    // [0.0, 0.9, 0.7], // Mint Green
    // [0.7, 0.9, 0.0], // Chartreuse
    // [0.9, 0.3, 0.3], // Salmon
    // [0.3, 0.9, 0.3], // Spring Green
    // [0.3, 0.3, 0.9], // Periwinkle
    // [0.9, 0.9, 0.3], // Light Yellow
    // [0.9, 0.3, 0.9], // Light Magenta
    // [0.3, 0.9, 0.9], // Light Cyan
    // [0.7, 0.5, 0.0], // Mustard
];

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

struct Curves {
    vertices: Vec<Vertex>, // len: MAX_CURVE_COUNT * MAX_CURVE_VERTICES
    vertex_count: u32,
    curve_count: u32,
}

impl Curves {
    fn get_indices(data: &mut [u32]) { // len: MAX_CURVE_COUNT * (MAX_CURVE_VERTICES - 1) * 2
        debug_assert_eq!(data.len(), MAX_CURVE_COUNT * (MAX_CURVE_VERTICES - 1) * 2);
        
        for c in 0..MAX_CURVE_COUNT {
            for v in 0..MAX_CURVE_VERTICES-1 {
                data[(c * (MAX_CURVE_VERTICES-1) + v) * 2] = v as u32;
                data[(c * (MAX_CURVE_VERTICES-1) + v) * 2 + 1] = v as u32 + 1;
            }
        }
    }

    fn get_ranges(&self, data: &mut [Option<Range<u32>>]) {
        debug_assert_eq!(data.len(), MAX_CURVE_COUNT);

        for c in 0..MAX_CURVE_COUNT {
            data[c] = if (c as u32) < self.curve_count {
                let start = (c * MAX_CURVE_VERTICES) as u32;
                let end = start + self.vertex_count;
                Some(start..end)
            } else {
                None
            }
        }
    }

    fn write(&mut self, data: &[Vec<Vertex>]) {
        self.curve_count = data.len() as u32;

        if self.curve_count == 0 {
            self.vertex_count = 0;
            return;
        }

        self.vertex_count = data[0].len() as u32;
        assert_ne!(self.vertex_count, 0);

        for (i, curve) in data.iter().enumerate() {
            for (j, vert) in curve.iter().enumerate() {
                self.vertices[MAX_CURVE_VERTICES * i + j] = *vert;
            }
        }
    }
}

struct Application {
    mouse_pos: Option<(f64, f64)>,
    curves: Curves,
    last_time: Instant,
    fps_lst: [f32; 100],
    fps_min: f32,
    fps: f32,
}

struct State<'a> {
    surface: wgpu::Surface<'a>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: &'a Window,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    curve_bind_groups: [wgpu::BindGroup; MAX_CURVE_COUNT],
    //
    app: Application,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window, curves: Curves) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let mut required_limits = wgpu::Limits::default();
        required_limits.max_bind_groups = MAX_CURVE_COUNT as u32;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits,
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: if VSYNC {
                wgpu::PresentMode::Fifo
            } else {
                surface_caps.present_modes[0]
            },
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&curves.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut indices = vec![0; MAX_CURVE_COUNT * (MAX_CURVE_VERTICES - 1) * 2];
        Curves::get_indices(indices.as_mut_slice());

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let curve_buffers: [wgpu::Buffer; MAX_CURVE_COUNT] = std::array::from_fn(|i| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(format!("Curve Buffer {i}").as_str()),
                contents: bytemuck::cast_slice(&COLORS),
                usage: wgpu::BufferUsages::UNIFORM,
            })
        });

        let curve_bind_group_layouts: [wgpu::BindGroupLayout; MAX_CURVE_COUNT] =
            std::array::from_fn(|i| {
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(format!("Curve Bind Group Layout {i}").as_str()),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some((size_of::<[f32; 4]>() as u64).try_into().unwrap()),
                        },
                        count: None,
                    }],
                })
            });

        let curve_bind_groups: [wgpu::BindGroup; MAX_CURVE_COUNT] = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(format!("Curve Bind Group {i}").as_str()),
                layout: &curve_bind_group_layouts[i],
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: curve_buffers[i].as_entire_binding(),
                }],
            })
        });

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                // bind_group_layouts: &curve_bind_group_layouts.iter().collect::<Vec<_>>(),
                bind_group_layouts: &std::array::from_fn::<_, MAX_CURVE_COUNT, _>(|i| &curve_bind_group_layouts[i]),
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            window,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            curve_bind_groups,
            app: Application {
                mouse_pos: None,
                curves,
                last_time: Instant::now(),
                fps_lst: [f32::NAN; 100],
                fps_min: f32::NAN,
                fps: f32::NAN,
            },
        }
    }

    pub fn window(&self) -> &'a Window {
        self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.app.mouse_pos = Some((position.x, position.y));
            }
            _ => {}
        }

        false
    }

    fn update(&mut self) {
        let time = Instant::now();
        let dur = time - self.app.last_time;
        self.app.last_time = time;

        let dt = dur.as_secs_f32();
        let fps = 1. / dt;

        self.app.fps_lst.rotate_right(1);
        self.app.fps_lst[0] = fps;
        self.app.fps_min = self
            .app
            .fps_lst
            .iter()
            .copied()
            .min_by(|a, b| match (a, b) {
                (x, _) if x.is_nan() => std::cmp::Ordering::Less,
                (_, y) if y.is_nan() => std::cmp::Ordering::Greater,
                _ => a.partial_cmp(b).unwrap(),
            })
            .unwrap();
        if self.app.fps.is_nan() {
            self.app.fps = fps;
        } else {
            self.app.fps = self.app.fps * 0.95 + fps * 0.05;
        }

        print!(
            "\rFramerate: {:.0} \t {:.0}        ",
            self.app.fps, self.app.fps_min
        );
        stdout().flush().unwrap();

        let dtheta = 60f32.to_radians() * dt;
        let x = dtheta.cos();
        let y = dtheta.sin();

        for v in &mut self.app.curves.vertices {
            let [a, b] = v.position;
            v.position = [a * x - b * y, a * y + b * x];
        }

        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&self.app.curves.vertices),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: self
                                    .app
                                    .mouse_pos
                                    .map_or(0.0, |x| x.0 / self.size.width as f64),
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.render_pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            let mut ranges = vec![None; MAX_CURVE_COUNT];
            self.app.curves.get_ranges(ranges.as_mut_slice());
            for (i, range) in ranges.iter().enumerate() {
                if range.is_none() {
                    break;
                }

                pass.set_bind_group(0, &self.curve_bind_groups[i], &[]);
            }
            for range in ranges.iter() {
                if range.is_none() {
                    break;
                }
                pass.draw_indexed(range.as_ref().unwrap().clone(), 0, 0..1);
            }
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    env_logger::init();

    let mut curves = Curves {
        vertices: vec![Vertex::default(); MAX_CURVE_COUNT * MAX_CURVE_VERTICES],
        vertex_count: 0,
        curve_count: 0,
    };

    curves.write(&vec![
        vec![
            Vertex {
                position: [-0.0868241, 0.49240386],
            },
            Vertex {
                position: [-0.49513406, 0.06958647],
            },
            Vertex {
                position: [-0.21918549, -0.44939706],
            },
            Vertex {
                position: [0.35966998, -0.3473291],
            },
            Vertex {
                position: [0.44147372, 0.2347359],
            },
        ],
        vec![
            Vertex {
                position: [-0.0968241, 0.59240386],
            },
            Vertex {
                position: [-0.59513406, 0.07958647],
            },
            Vertex {
                position: [-0.31918549, -0.54939706],
            },
            Vertex {
                position: [0.45966998, -0.4473291],
            },
            Vertex {
                position: [0.54147372, 0.3347359],
            },
        ],
    ]);

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(&window, curves).await;

    event_loop
        .run(move |event, control_flow| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            state.update();
                            match state.render() {
                                Ok(_) => {}
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                                Err(e) => eprintln!("{e:?}"),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                // RedrawRequested will only trigger once unless we manually
                // request it.
                state.window().request_redraw();
            }
            _ => {}
        })
        .unwrap();
}
