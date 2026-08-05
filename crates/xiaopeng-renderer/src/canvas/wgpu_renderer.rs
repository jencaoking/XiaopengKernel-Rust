use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::DisplayCommand;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

pub struct WgpuRenderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl WgpuRenderer {
    pub async fn new(window: Arc<winit::window::Window>) -> Result<Self, String> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        
        let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
        
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }).await.map_err(|e| e.to_string())?;
        
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        }).await.map_err(|e| e.to_string())?;
        
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
            
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).unwrap();
        surface.configure(&device, &config);
        
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            ..Default::default()
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: std::num::NonZero::new(0),
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
        })
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    pub fn render(&mut self, display_list: &crate::DisplayList) -> Result<(), String> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Timeout => return Err("Surface timeout".to_string()),
            wgpu::CurrentSurfaceTexture::Occluded => return Err("Surface occluded".to_string()),
            wgpu::CurrentSurfaceTexture::Outdated => return Err("Surface outdated".to_string()),
            wgpu::CurrentSurfaceTexture::Lost => return Err("Surface lost".to_string()),
            wgpu::CurrentSurfaceTexture::Validation => return Err("Surface validation error".to_string()),
        };
        
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut current_index = 0;
        
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        
        // This is a bit dirty, mapping DisplayList (which uses absolute coordinates) to vertices
        // In reality we should iterate through DisplayCommands
        for command in &display_list.commands {
            if let DisplayCommand::DrawRect { rect, color } = command {
                let x0 = (rect.x / width) * 2.0 - 1.0;
                let y0 = 1.0 - (rect.y / height) * 2.0;
                let x1 = ((rect.x + rect.width) / width) * 2.0 - 1.0;
                let y1 = 1.0 - ((rect.y + rect.height) / height) * 2.0;

                let rgba = [
                    color.r as f32 / 255.0,
                    color.g as f32 / 255.0,
                    color.b as f32 / 255.0,
                    color.a as f32 / 255.0,
                ];

                vertices.push(Vertex { pos: [x0, y0], color: rgba });
                vertices.push(Vertex { pos: [x1, y0], color: rgba });
                vertices.push(Vertex { pos: [x1, y1], color: rgba });
                vertices.push(Vertex { pos: [x0, y1], color: rgba });

                indices.push(current_index);
                indices.push(current_index + 1);
                indices.push(current_index + 2);
                indices.push(current_index);
                indices.push(current_index + 2);
                indices.push(current_index + 3);
                current_index += 4;
            }
        }
        
        let has_damage = display_list.damage_rect.is_some();
        let load_op = if has_damage {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            })
        };
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: std::num::NonZero::new(0),
            });
            
            if let Some(rect) = display_list.damage_rect {
                let sc_x = (rect.x.max(0.0)) as u32;
                let sc_y = (rect.y.max(0.0)) as u32;
                let sc_w = (rect.width.max(0.0)) as u32;
                let sc_h = (rect.height.max(0.0)) as u32;
                
                // Clamp to window bounds to avoid panics
                let sc_x = sc_x.min(width as u32);
                let sc_y = sc_y.min(height as u32);
                let sc_w = sc_w.min(width as u32 - sc_x);
                let sc_h = sc_h.min(height as u32 - sc_y);
                
                if sc_w > 0 && sc_h > 0 {
                    render_pass.set_scissor_rect(sc_x, sc_y, sc_w, sc_h);
                }
            }
            
            if !vertices.is_empty() {
                let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                
                let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
            }
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);
        
        Ok(())
    }
}
