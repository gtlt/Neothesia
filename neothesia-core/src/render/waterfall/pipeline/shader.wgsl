struct ViewUniform {
    transform: mat4x4<f32>,
    size: vec2<f32>,
    scale: f32,
}

struct TimeUniform {
    time: f32,
    speed: f32,
    // Top left corner of the keyboard
    keyboard: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> view_uniform: ViewUniform;

@group(1) @binding(0)
var<uniform> time_uniform: TimeUniform;

struct Vertex {
    @location(0) position: vec2<f32>,
}

struct NoteInstance {
    @location(1) n_position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) color: vec3<f32>,
    @location(4) radius: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,

    @location(0) src_position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec3<f32>,
    @location(3) radius: f32,
    @location(4) note_pos: vec2<f32>,
}

@vertex
fn vs_main(vertex: Vertex, note: NoteInstance) -> VertexOutput {
    let speed = time_uniform.speed;

    let size = vec2<f32>(note.size.x * view_uniform.scale, note.size.y * abs(speed));

    let keyboard = time_uniform.keyboard * view_uniform.scale;

    var pos = vec2<f32>(keyboard.x + note.n_position.x * view_uniform.scale, keyboard.y);

    if speed > 0.0 {
        // If notes are falling from top to down, we need to adjust the position,
        // as their start is on bottom of the quad rather than top
        pos.y -= size.y;
    }

    // Offset position by playback time
    pos.y -= (note.n_position.y - time_uniform.time) * speed;

    // 1px of padding to give us some space for anti-aliasing
    let pad = 1.0;
    let quad_pos = pos - vec2<f32>(pad, pad);
    let quad_size = size + vec2<f32>(pad, pad) * 2.0;

    let transform = mat4x4<f32>(
        vec4<f32>(quad_size.x, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, quad_size.y, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(quad_pos.x, quad_pos.y, 0.0, 1.0)
    );

    var out: VertexOutput;
    out.position = view_uniform.transform * transform * vec4<f32>(vertex.position, 0.0, 1.0);
    out.note_pos = pos;

    out.src_position = vertex.position;
    out.size = size;
    out.color = note.color;
    out.radius = note.radius * view_uniform.scale;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let half_size = in.size * 0.5;
    // From the center of the note
    let p = in.position.xy - in.note_pos - half_size;

    // Distance to the edge of the rounded box, negative inside
    let radius = min(in.radius, min(half_size.x, half_size.y));
    let q = abs(p) - half_size + vec2<f32>(radius);
    let dist = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;

    let alpha = 1.0 - smoothstep(-0.5, 0.5, dist);

    // Shaded on the gamma encoded color, where a step looks the same on every hue
    var color = pow(in.color, vec3<f32>(1.0 / 2.2));

    // A note is a bar lit from the left: its sides turn away from the light,
    // the right one more than the left one
    let across = p.x / half_size.x;
    color *= 1.0 - 0.14 * smoothstep(0.3, 1.0, abs(across + 0.15));

    // A darker rim keeps two notes that touch apart
    color *= 1.0 - 0.22 * smoothstep(-2.5, -0.5, dist);

    return vec4<f32>(pow(color, vec3<f32>(2.2)), alpha);
}
