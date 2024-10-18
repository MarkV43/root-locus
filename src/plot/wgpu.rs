use std::{io::{stdout, Write}, ops::Range, time::Instant};

use wgpu::{include_wgsl, util::DeviceExt};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const VSYNC: bool = false;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: u32,
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
    vertices: Vec<Vertex>,
    curve_indices: Vec<usize>,
}

impl Curves {
    fn get_indices(&self) -> Vec<u16> {
        let nc = self.curve_indices.len();
        let nv = self.vertices.len();
        let n = (nv - nc) * 2;
        let mut buffer = vec![0; n];
        let mut p = 0;
        let mut ind = 0;

        for (i, b) in buffer.chunks_exact_mut(2).enumerate() {
            b[0] = p;
            b[1] = p + 1;
            if ind + 1 < nc && self.curve_indices[ind + 1] <= i + 2 {
                p += 2;
                ind += 1;
            } else {
                p += 1;
            }
        }

        buffer
    }

    fn get_ranges(&self) -> Vec<Range<u32>> {
        let nc = self.curve_indices.len();
        let nv = self.vertices.len();

        let mut buffer = vec![0..0; nc];

        let mut ind = self
            .curve_indices
            .iter()
            .copied()
            .chain(std::iter::once(nv));
        let mut start = ind.next().unwrap() as u32;
        let mut prev = start;

        for i in 1..nc + 1 {
            let end = ind.next().unwrap() as u32;
            let diff = end - prev;
            prev = end;
            let end = start + (diff - 1) * 2;
            buffer[i - 1] = start..end;
            start = end;
        }

        buffer
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
    //
    app: Application,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window, curves: Curves) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
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
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
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

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&curves.get_indices()),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
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
                cull_mode: Some(wgpu::Face::Back),
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
        self.app.fps_min = self.app.fps_lst.iter().copied().min_by(|a, b| 
            match (a, b) {
                (x, _) if x.is_nan() => std::cmp::Ordering::Less,
                (_, y) if y.is_nan() => std::cmp::Ordering::Greater,
                _ => a.partial_cmp(b).unwrap(),
            }
        ).unwrap();
        if self.app.fps.is_nan() {
            self.app.fps = fps;
        } else {
            self.app.fps = self.app.fps * 0.95 + fps * 0.05;
        }

        print!("\rFramerate: {:.0} \t {:.0}        ", self.app.fps, self.app.fps_min);
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
        self.queue.write_buffer(
            &self.index_buffer,
            0,
            bytemuck::cast_slice(&self.app.curves.get_indices()),
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

            for range in self.app.curves.get_ranges() {
                pass.draw_indexed(range, 0, 0..1);
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

    let curves = Curves {
        vertices: vec![
            Vertex {
                position: [-0.0868241, 0.49240386],
                color: 0,
            },
            Vertex {
                position: [-0.49513406, 0.06958647],
                color: 0,
            },
            Vertex {
                position: [-0.21918549, -0.44939706],
                color: 0,
            },
            Vertex {
                position: [0.35966998, -0.3473291],
                color: 0,
            },
            Vertex {
                position: [0.44147372, 0.2347359],
                color: 0,
            },
            //
            Vertex {
                position: [-0.0968241, 0.59240386],
                color: 1,
            },
            Vertex {
                position: [-0.59513406, 0.07958647],
                color: 1,
            },
            Vertex {
                position: [-0.31918549, -0.54939706],
                color: 1,
            },
            Vertex {
                position: [0.45966998, -0.4473291],
                color: 1,
            },
            Vertex {
                position: [0.54147372, 0.3347359],
                color: 1,
            },
        ],
        curve_indices: vec![0, 5],
    };

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
