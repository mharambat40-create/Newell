// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Batched screen-space strokes for the active Sketch draft.

use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug)]
pub struct SketchStroke {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub selected: bool,
}

pub(crate) struct StrokeFrame<'a> {
    pub strokes: &'a [SketchStroke],
    pub surface: [u32; 2],
    pub color: [f32; 3],
    pub scale: f32,
}

pub(crate) struct SketchPipeline(wgpu::RenderPipeline);

impl SketchPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sketch strokes"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sketch.wgsl").into()),
        });
        Self(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Sketch strokes"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 20,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(format.into())],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            }),
        )
    }

    pub fn draw(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        scene: &crate::scene::Scene,
        frame: StrokeFrame<'_>,
    ) {
        let StrokeFrame {
            strokes,
            surface,
            color,
            scale,
        } = frame;
        if strokes.is_empty() || surface.contains(&0) {
            return;
        }
        let mut vertices = Vec::with_capacity(strokes.len() * 120);
        for stroke in strokes {
            let [ax, ay] = stroke.start;
            let [bx, by] = stroke.end;
            let length = (bx - ax).hypot(by - ay);
            if !length.is_finite() || length == 0.0 {
                continue;
            }
            let half = if stroke.selected { 1.25 } else { 0.75 } * scale;
            let x = (by - ay) / length * half;
            let y = (ax - bx) / length * half;
            let points = [
                [ax + x, ay + y],
                [ax - x, ay - y],
                [bx + x, by + y],
                [bx - x, by - y],
            ];
            for [px, py] in [
                points[0], points[1], points[2], points[2], points[1], points[3],
            ] {
                let position = [
                    2.0 * (px / surface[0] as f32) - 1.0,
                    1.0 - 2.0 * (py / surface[1] as f32),
                ];
                vertices.extend(position.into_iter().chain(color).flat_map(f32::to_ne_bytes));
            }
        }
        let Ok(count) = u32::try_from(vertices.len() / 20) else {
            return;
        };
        if count == 0 {
            return;
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sketch stroke vertices"),
            contents: &vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sketch strokes"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.0);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for rect in crate::exposed_regions(scene.viewport, &scene.occlusions) {
            pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
            pass.draw(0..count, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "needs a GPU"]
    fn sketch_stroke_pipeline_validates() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = crate::renderer::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        )
        .unwrap();
        let (device, _) =
            crate::renderer::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .unwrap();
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _pipeline = super::SketchPipeline::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert!(crate::renderer::block_on(device.pop_error_scope()).is_none());
    }
}
