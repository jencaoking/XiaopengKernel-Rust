use std::num::NonZero;
use pollster::FutureExt;
use wgpu::util::DeviceExt;
use xiaopeng_layout::LayoutBox;
use crate::canvas::wgpu_shared::{Vertex, create_render_pipeline};

pub struct GpuCanvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl GpuCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize], // RGBA
        }
    }
}

pub fn render_display_list_gpu(display_list: &[&LayoutBox], width: u32, height: u32) -> Result<GpuCanvas, String> {
    // 1. Initialize WGPU Instance and Device
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        ..Default::default()
    }).block_on().map_err(|e| e.to_string())?; 

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        memory_hints: wgpu::MemoryHints::Performance,
        ..Default::default()
    }).block_on().map_err(|e| e.to_string())?;

    // 2. Create Target Texture for headless rendering
    let texture_size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let render_target = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        label: Some("Render Target"),
        view_formats: &[],
    });
    let render_target_view = render_target.create_view(&wgpu::TextureViewDescriptor::default());

    // 3. Create Pipeline
    let render_pipeline = create_render_pipeline(&device, wgpu::TextureFormat::Rgba8Unorm);

    // 4. Generate Vertices from DisplayList
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut current_index = 0;

    for box_ in display_list {
        let rect = box_.dimensions.border_box();
        let color = box_.style.background_color;
        
        let x0 = (rect.x / width as f32) * 2.0 - 1.0;
        let y0 = 1.0 - (rect.y / height as f32) * 2.0;
        let x1 = ((rect.x + rect.width) / width as f32) * 2.0 - 1.0;
        let y1 = 1.0 - ((rect.y + rect.height) / height as f32) * 2.0;

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

    if vertices.is_empty() {
        return Ok(GpuCanvas::new(width, height));
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // 5. Encode Render Commands
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: NonZero::new(0),
        });

        rpass.set_pipeline(&render_pipeline);
        rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
        rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        rpass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }

    // 6. Read back pixels to CPU Buffer
    let bytes_per_pixel = 4;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (padded_bytes_per_row * height) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &render_target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        texture_size,
    );

    let submission = queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).expect("Unwrap failed");
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    }).expect("Unwrap failed");
    rx.recv().expect("Unwrap failed").expect("Unwrap failed");

    let mut result = GpuCanvas::new(width, height);
    {
        let data = buffer_slice.get_mapped_range().expect("Unwrap failed");
        // Remove padding
        for y in 0..height {
            let src_start = (y * padded_bytes_per_row) as usize;
            let src_end = src_start + unpadded_bytes_per_row as usize;
            let dst_start = (y * unpadded_bytes_per_row) as usize;
            let dst_end = dst_start + unpadded_bytes_per_row as usize;
            result.pixels[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
        }
    }
    output_buffer.unmap();

    Ok(result)
}
