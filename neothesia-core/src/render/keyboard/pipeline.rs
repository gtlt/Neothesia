use bytemuck::{Pod, Zeroable};
use wgpu::vertex_attr_array;
use wgpu_jumpstart::{Gpu, Instances, Shape, TransformUniform, Uniform, wgpu};

/// What an instance draws, the shader has the same list.
pub mod kind {
    pub const WHITE_KEY: f32 = 0.0;
    pub const BLACK_KEY: f32 = 1.0;
    /// Shadow of a black key on the white ones
    pub const BLACK_KEY_SHADOW: f32 = 2.0;
    /// What shows between the keys
    pub const BED: f32 = 3.0;
    /// Strip of felt along the back of the keys
    pub const FELT: f32 = 4.0;
    /// Block at one end of the keyboard
    pub const CHEEK: f32 = 5.0;
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, PartialEq)]
pub struct KeyInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    /// Linear RGB, and in alpha how much of it the key takes
    pub tint: [f32; 4],
    /// Kind, then what the kind needs:
    /// - keys and their shadow: how far down the key is, from 0 to 1
    /// - shadow: also the width and height of the key
    /// - cheek: 0 for the left one, 1 for the right one
    pub params: [f32; 4],
}

impl KeyInstance {
    fn attributes() -> [wgpu::VertexAttribute; 4] {
        vertex_attr_array!(
            1 => Float32x2,
            2 => Float32x2,
            3 => Float32x4,
            4 => Float32x4
        )
    }

    fn layout(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<KeyInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes,
        }
    }
}

/// Draws the whole keyboard in one call, every key is shaded by the fragment shader.
pub struct KeyboardPipeline {
    render_pipeline: wgpu::RenderPipeline,
    quad: Shape,
    instances: Instances<KeyInstance>,
    transform_uniform_bind_group: wgpu::BindGroup,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl KeyboardPipeline {
    pub fn new(gpu: &Gpu, transform_uniform: &Uniform<TransformUniform>, size_hint: usize) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("KeyboardPipeline::shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                    "./shader.wgsl"
                ))),
            });

        let render_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&transform_uniform.bind_group_layout)],
                    immediate_size: 0,
                });

        let target = wgpu::ColorTargetState {
            // Shader outputs premultiplied colors
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            ..wgpu_jumpstart::default_color_target_state(gpu.texture_format)
        };

        let attributes = KeyInstance::attributes();

        let render_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: Some(&render_pipeline_layout),
                fragment: Some(wgpu_jumpstart::default_fragment(&shader, &[Some(target)])),
                ..wgpu_jumpstart::default_render_pipeline(wgpu_jumpstart::default_vertex(
                    &shader,
                    &[
                        Some(Shape::layout()),
                        Some(KeyInstance::layout(&attributes)),
                    ],
                ))
            });

        Self {
            render_pipeline,
            quad: Shape::new_quad(&gpu.device),
            instances: Instances::new(&gpu.device, size_hint),
            transform_uniform_bind_group: transform_uniform.bind_group.clone(),
            device: gpu.device.clone(),
            queue: gpu.queue.clone(),
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.instances.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.transform_uniform_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.quad.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances.buffer.slice(..));

        render_pass.set_index_buffer(self.quad.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.draw_indexed(0..self.quad.indices_len, 0, 0..self.instances.len());
    }

    pub fn instances(&mut self) -> &mut Vec<KeyInstance> {
        &mut self.instances.data
    }

    /// Upload instances to the GPU
    pub fn prepare(&mut self) {
        self.instances.update(&self.device, &self.queue);
    }
}
