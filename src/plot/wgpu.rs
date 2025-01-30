use core::f32;
use std::{
    io::{stdout, Write},
    time::Instant,
};

use num::complex::Complex32;
use rust_lab::{
    midware::{
        curves::{Curves, Point},
        Midware,
    },
    polynomials::roots::PolynomialRoot,
};
use wgpu::{include_wgsl, util::DeviceExt};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const VSYNC: bool = false;

type RawColor = [f32; 3];
type Color = [f32; 64];

const RAW_COLORS: &[RawColor] = &[
    [1.0, 0.0, 0.0], // Red
    [0.0, 1.0, 0.0], // Green
    [0.0, 0.0, 1.0], // Blue
    [1.0, 1.0, 0.0], // Yellow
    [1.0, 0.0, 1.0], // Magenta
    [0.0, 1.0, 1.0], // Cyan
    [0.5, 0.0, 0.0], // Dark Red
    [0.0, 0.5, 0.0], // Dark Green
    [0.0, 0.0, 0.5], // Dark Blue
    [1.0, 0.5, 0.0], // Orange
    [0.5, 0.0, 0.5], // Purple
    [0.0, 0.5, 0.5], // Teal
    [0.8, 0.4, 0.0], // Brown
    [0.0, 0.4, 0.8], // Sky Blue
    [0.4, 0.8, 0.0], // Lime Green
    [0.8, 0.0, 0.4], // Pink
    [0.6, 0.3, 0.3], // Light Brown
    [0.3, 0.6, 0.3], // Light Green
    [0.3, 0.3, 0.6], // Light Blue
    [0.9, 0.7, 0.0], // Gold
    [0.7, 0.0, 0.9], // Violet
    [0.0, 0.7, 0.9], // Aquamarine
    [0.9, 0.0, 0.7], // Hot Pink
    [0.0, 0.9, 0.7], // Mint Green
    [0.7, 0.9, 0.0], // Chartreuse
    [0.9, 0.3, 0.3], // Salmon
    [0.3, 0.9, 0.3], // Spring Green
    [0.3, 0.3, 0.9], // Periwinkle
    [0.9, 0.9, 0.3], // Light Yellow
    [0.9, 0.3, 0.9], // Light Magenta
    [0.3, 0.9, 0.9], // Light Cyan
    [0.7, 0.5, 0.0], // Mustard
];

const MAX_CURVE_COUNT: usize = RAW_COLORS.len();
const MAX_CURVE_VERTICES: usize = 1024;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

impl From<Complex32> for Vertex {
    fn from(value: Complex32) -> Self {
        Self {
            position: [value.re, value.im],
        }
    }
}

impl Point<f32> for Vertex {
    fn x(&self) -> f32 {
        self.position[0]
    }

    fn y(&self) -> f32 {
        self.position[1]
    }
}

struct Application {
    mouse_pos: Option<(f64, f64)>,
    curves: Curves<MAX_CURVE_COUNT, MAX_CURVE_VERTICES, Vertex, f32>,
    midware: Midware<f32>,
    last_time: Instant,
    fps: FramerateData,
}

struct FramerateData {
    fps: f32,
    fps_hst: [f32; 128],
    fps_hst_idx: usize,
    fps_min: f32,
    fps_avg: f32,
}

impl FramerateData {
    fn new() -> Self {
        Self {
            fps: f32::NAN,
            fps_hst: [f32::NAN; 128],
            fps_hst_idx: 0,
            fps_min: f32::NAN,
            fps_avg: f32::NAN,
        }
    }

    fn update(&mut self, fps: f32) {
        self.fps = fps;
        self.fps_hst[self.fps_hst_idx] = fps;
        self.fps_hst_idx = (self.fps_hst_idx + 1) % self.fps_hst.len();
        let mut min = f32::INFINITY;
        let mut cnt = 0;
        let mut avg = 0.0;
        for &val in self.fps_hst.iter().filter(|x| x.is_finite()) {
            if val < min {
                min = val;
            }
            avg += val;
            cnt += 1;
        }
        avg /= cnt as f32;
        self.fps_min = min;
        self.fps_avg = avg;
    }
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
    color_bind_group: wgpu::BindGroup,
    //
    app: Application,
}

impl<'a> State<'a> {
    async fn new(
        window: &'a Window,
        midware: Midware<f32>,
        curves: Curves<MAX_CURVE_COUNT, MAX_CURVE_VERTICES, Vertex, f32>,
    ) -> Self {
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

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
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
            contents: bytemuck::cast_slice(curves.get_vertices()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut indices = vec![0; MAX_CURVE_COUNT * (MAX_CURVE_VERTICES - 1) * 2];
        Curves::<MAX_CURVE_COUNT, MAX_CURVE_VERTICES, Vertex, f32>::get_indices(
            indices.as_mut_slice(),
        );

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let padded_colors: Vec<Color> = RAW_COLORS
            .iter()
            .map(|col| {
                let mut padded = [0.0; 64];
                padded[..col.len()].copy_from_slice(col);
                padded
            })
            .collect();

        let color_buffer: wgpu::Buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Color Buffer"),
                contents: bytemuck::cast_slice(&padded_colors),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let color_bind_group_layout: wgpu::BindGroupLayout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Color Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let color_bind_group: wgpu::BindGroup =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Curve Bind Group"),
                layout: &color_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &color_buffer,
                        offset: 0,
                        size: Some((size_of::<Color>() as u64).try_into().unwrap()),
                    }),
                }],
            });

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                // bind_group_layouts: &curve_bind_group_layouts.iter().collect::<Vec<_>>(),
                bind_group_layouts: &[&color_bind_group_layout],
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
            color_bind_group,
            app: Application {
                mouse_pos: None,
                midware,
                curves,
                last_time: Instant::now(),
                fps: FramerateData::new(),
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

        self.app.fps.update(fps);

        print!(
            "\rFramerate: {:.0} \t {:.0} \t {:.0}\n",
            self.app.fps.fps, self.app.fps.fps_min, self.app.fps.fps_avg
        );
        stdout().flush().unwrap();

        self.app.midware.update(&mut self.app.curves);

        /* let dtheta = 60f32.to_radians() * dt;
        let x = dtheta.cos();
        let y = dtheta.sin();

        let mut ranges = vec![None; MAX_CURVE_COUNT];
        self.app.curves.get_vertex_ranges(ranges.as_mut_slice());

        let vertices = self.app.curves.get_vertices_mut();

        for r in ranges {
            if r.is_none() {
                break;
            }

            for v in &mut vertices[r.unwrap()] {
                let [a, b] = v.position;
                v.position = [a * x - b * y, a * y + b * x];
            }
        }

        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(vertices),
        ); */
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

            let mut indices = vec![0; MAX_CURVE_COUNT * (MAX_CURVE_VERTICES - 1) * 2];
            Curves::<MAX_CURVE_COUNT, MAX_CURVE_VERTICES, Vertex, f32>::get_indices(
                indices.as_mut_slice(),
            );

            let mut ranges = vec![None; MAX_CURVE_COUNT];
            self.app.curves.get_index_ranges(ranges.as_mut_slice());
            for (i, range) in ranges.iter().enumerate() {
                if range.is_none() {
                    break;
                }

                pass.set_bind_group(
                    0,
                    &self.color_bind_group,
                    &[i as u32 * size_of::<Color>() as u32],
                );
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

    let mut midware = Midware::new();

    println!("1");

    let zeros = midware.zeros_mut();
    zeros.push(PolynomialRoot::RealSingle(-3.0));

    let poles = midware.poles_mut();
    poles.push(PolynomialRoot::RealSingle(-1.0));
    poles.push(PolynomialRoot::ComplexPair(Complex32::new(-2.0, 1.0)));

    println!("2");

    let mut curves = Curves::new();
    midware.update(&mut curves);

    println!("3");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    println!("4");

    let mut state = State::new(&window, midware, curves).await;

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
