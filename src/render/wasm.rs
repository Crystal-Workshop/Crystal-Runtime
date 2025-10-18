use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use bytemuck::bytes_of;
use glam::{Mat3, Mat4};
use log::error;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::{Window, WindowId};

use super::{
    shared::{DEFAULT_CUBE_INDICES, DEFAULT_CUBE_VERTICES, SHADER},
    CameraParams, LightParams,
};
use crate::{CGameArchive, ObjMesh, SceneObject};

pub struct Renderer {
    window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    depth: DepthBuffer,
    pipeline: wgpu::RenderPipeline,
    global_buffer: wgpu::Buffer,
    global_bind_group: wgpu::BindGroup,
    object_layout: wgpu::BindGroupLayout,
    mesh_cache: HashMap<String, MeshBuffers>,
    missing_meshes: HashSet<String>,
    archive: Arc<CGameArchive>,
    default_mesh: MeshBuffers,
}

impl Renderer {
    pub async fn new(window: Window, archive: Arc<CGameArchive>) -> Result<Self> {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Err(anyhow!("window has zero area"));
        }

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            dx12_shader_compiler: Default::default(),
        });
        let surface = unsafe { instance.create_surface(&window) }?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("failed to acquire GPU adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    label: Some("renderer-device"),
                },
                None,
            )
            .await
            .context("failed to create GPU device")?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let depth = DepthBuffer::create(&device, config.width, config.height);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("renderer-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let global_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("global-bind-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(std::mem::size_of::<GlobalUniform>() as u64)
                            .unwrap(),
                    ),
                },
                count: None,
            }],
        });

        let object_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("object-bind-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(std::mem::size_of::<ObjectConstants>() as u64)
                            .unwrap(),
                    ),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("renderer-pipeline-layout"),
            bind_group_layouts: &[&global_layout, &object_layout],
            push_constant_ranges: &[],
        });

        let global_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("global-uniform"),
            size: std::mem::size_of::<GlobalUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("global-bind-group"),
            layout: &global_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: global_buffer.as_entire_binding(),
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("renderer-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: (6 * std::mem::size_of::<f32>()) as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: (3 * std::mem::size_of::<f32>()) as u64,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthBuffer::FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let default_mesh = MeshBuffers::from_mesh(
            &device,
            &ObjMesh {
                vertices: DEFAULT_CUBE_VERTICES.to_vec(),
                indices: DEFAULT_CUBE_INDICES.to_vec(),
            },
            "default-cube",
        );

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            depth,
            pipeline,
            global_buffer,
            global_bind_group,
            object_layout,
            mesh_cache: HashMap::new(),
            missing_meshes: HashSet::new(),
            archive,
            default_mesh,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth = DepthBuffer::create(&self.device, new_size.width, new_size.height);
    }

    pub fn update_globals(&self, camera: &CameraParams, light: &LightParams) {
        let uniform = GlobalUniform {
            view_proj: camera.view_proj.to_cols_array_2d(),
            camera_position: camera.position.extend(1.0).into(),
            light_position: light.position.extend(1.0).into(),
            light_color: light.color.extend(light.intensity).into(),
        };
        self.queue
            .write_buffer(&self.global_buffer, 0, bytes_of(&uniform));
    }

    pub fn render(&mut self, objects: &[SceneObject]) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("renderer-encoder"),
            });

        let mut draw_list = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            if !object_wants_mesh(object) {
                continue;
            }
            if let Some(name) = object.mesh.as_deref() {
                self.ensure_mesh_loaded(name);
                draw_list.push((Some(name.to_string()), index));
            } else {
                draw_list.push((None, index));
            }
        }

        let mut bind_groups = Vec::new();
        for (mesh_name, obj_index) in draw_list.iter() {
            let object = &objects[*obj_index];
            let model = object_model_matrix(object);
            let normal = Mat3::from_mat4(model).inverse().transpose();
            let constants = ObjectConstants {
                model: model.to_cols_array_2d(),
                normal: mat3_to_3x4(normal),
                color: object.color.extend(1.0).into(),
            };

            let object_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("object-uniform"),
                    contents: bytemuck::bytes_of(&constants),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

            let object_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.object_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: object_buffer.as_entire_binding(),
                }],
                label: Some("object-bind-group"),
            });

            bind_groups.push((mesh_name.clone(), object_bind_group));
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.03,
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.global_bind_group, &[]);

            for ((mesh_name, _), (_, bind_group)) in draw_list.iter().zip(bind_groups.iter()) {
                let mesh = match mesh_name.as_ref() {
                    Some(name) => self.mesh_cache.get(name).unwrap_or(&self.default_mesh),
                    None => &self.default_mesh,
                };

                pass.set_vertex_buffer(0, mesh.vertex.slice(..));
                pass.set_index_buffer(mesh.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_bind_group(1, bind_group, &[]);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn ensure_mesh_loaded(&mut self, name: &str) {
        if self.mesh_cache.contains_key(name) || self.missing_meshes.contains(name) {
            return;
        }
        match self.load_mesh(name) {
            Ok(mesh) => {
                self.mesh_cache.insert(name.to_string(), mesh);
            }
            Err(err) => {
                error!("failed to load mesh {name}: {err:?}");
                self.missing_meshes.insert(name.to_string());
            }
        }
    }

    fn load_mesh(&self, name: &str) -> Result<MeshBuffers> {
        let bytes = self
            .archive
            .extract_file(name)
            .with_context(|| format!("unable to extract {name} from archive"))?;
        let contents =
            String::from_utf8(bytes).with_context(|| format!("{name} is not valid UTF-8"))?;
        let mesh = crate::load_obj_from_str(&contents)
            .with_context(|| format!("failed to parse OBJ mesh {name}"))?;
        Ok(MeshBuffers::from_mesh(&self.device, &mesh, name))
    }
}

struct MeshBuffers {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    index_count: u32,
}

impl MeshBuffers {
    fn from_mesh(device: &wgpu::Device, mesh: &ObjMesh, label: &str) -> Self {
        let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{label}-vertex")),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{label}-index")),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex,
            index,
            index_count: mesh.indices.len() as u32,
        }
    }
}

struct DepthBuffer {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl DepthBuffer {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

    fn create(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("depth-buffer"),
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalUniform {
    view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    light_position: [f32; 4],
    light_color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ObjectConstants {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 3],
    color: [f32; 4],
}

fn mat3_to_3x4(mat: Mat3) -> [[f32; 4]; 3] {
    [
        [mat.x_axis.x, mat.x_axis.y, mat.x_axis.z, 0.0],
        [mat.y_axis.x, mat.y_axis.y, mat.y_axis.z, 0.0],
        [mat.z_axis.x, mat.z_axis.y, mat.z_axis.z, 0.0],
    ]
}

fn object_model_matrix(object: &SceneObject) -> Mat4 {
    let translation = Mat4::from_translation(object.position);
    let rotation = Mat4::from_rotation_z(object.rotation.z.to_radians())
        * Mat4::from_rotation_y(object.rotation.y.to_radians())
        * Mat4::from_rotation_x(object.rotation.x.to_radians());
    let scale = Mat4::from_scale(object.scale);
    translation * rotation * scale
}

fn object_wants_mesh(object: &SceneObject) -> bool {
    object.object_type == "mesh" || object.mesh.is_some()
}
