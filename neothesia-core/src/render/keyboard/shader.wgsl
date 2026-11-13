struct ViewUniform {
    transform: mat4x4<f32>,
    size: vec2<f32>,
    scale: f32,
}

@group(0) @binding(0)
var<uniform> view_uniform: ViewUniform;

struct Vertex {
    @location(0) position: vec2<f32>,
}

struct KeyInstance {
    @location(1) k_position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) params: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,

    // Pixels from the top left corner of the instance
    @location(0) local: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) params: vec4<f32>,
}

// Same list as `kind` in pipeline.rs
const WHITE_KEY: f32 = 0.0;
const BLACK_KEY: f32 = 1.0;
const BLACK_KEY_SHADOW: f32 = 2.0;
const BED: f32 = 3.0;
const FELT: f32 = 4.0;
const CHEEK: f32 = 5.0;

@vertex
fn vs_main(vertex: Vertex, key: KeyInstance) -> VertexOutput {
    let position = key.k_position * view_uniform.scale;
    let size = key.size * view_uniform.scale;

    // One more pixel all around, for anti-aliasing
    let pad = 1.0;
    let local = vertex.position * (size + vec2<f32>(pad * 2.0)) - vec2<f32>(pad);

    var out: VertexOutput;
    out.position = view_uniform.transform * vec4<f32>(position + local, 0.0, 1.0);
    out.local = local;
    out.size = size;
    out.tint = key.tint;
    out.params = vec4<f32>(key.params.xy, key.params.zw * view_uniform.scale);
    return out;
}

// All the shading is done on gamma encoded colors, where "a bit darker" means
// the same thing for a light and for a dark color.
fn to_linear(color: vec3<f32>) -> vec3<f32> {
    return pow(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(2.2));
}

fn to_gamma(color: vec3<f32>) -> vec3<f32> {
    return pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

// Distance to the edge of a rounded box, negative inside.
// `p` is measured from the top left corner of the box.
fn rounded_box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let half_size = size * 0.5;
    let q = abs(p - half_size) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// A key is square at the back and rounded at the front
fn key_shape(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let is_front = p.y > size.y * 0.5;
    return rounded_box(p, size, select(0.0, radius, is_front));
}

fn white_key(p: vec2<f32>, size: vec2<f32>, tint: vec4<f32>, travel: f32) -> vec4<f32> {
    let w = size.x;
    let h = size.y;

    // Keys do not touch, a dark slit stays between two of them
    let gap = max(1.0, floor(w * 0.02));
    let q = p - vec2<f32>(gap * 0.5, 0.0);
    let body = size - vec2<f32>(gap, 0.0);

    let dist = key_shape(q, body, clamp(w * 0.07, 1.5, 14.0));
    let alpha = 1.0 - smoothstep(-0.5, 0.5, dist);

    // 0 at the back of the key, 1 at the front
    let v = clamp(q.y / h, 0.0, 1.0);

    // Ivory, and a color that fades a little towards the player
    var color = vec3<f32>(0.972, 0.965, 0.942);
    color = mix(color, to_gamma(tint.rgb), tint.a * mix(1.0, 0.8, v));

    // The fallboard shades the back of the keys
    color *= 1.0 - 0.30 * exp(-q.y / (h * 0.045));

    // Edges are rounded and catch less light, the one away from the light even less
    let from_left = q.x;
    let from_right = body.x - q.x;
    let edge = max(1.5, w * 0.05);
    color *= 1.0 - 0.10 * (1.0 - smoothstep(0.0, edge, from_left));
    color *= 1.0 - 0.18 * (1.0 - smoothstep(0.0, edge * 1.4, from_right));

    // A key that is down sits below its neighbours, they shade it
    let from_side = min(from_left, from_right);
    color *= 1.0 - travel * (0.08 + 0.30 * (1.0 - smoothstep(0.0, w * 0.25, from_side)));

    // The front face of the key, less and less of it shows as the key goes down
    let lip = clamp(h * 0.035, 3.0, w * 0.25) * (1.0 - 0.7 * travel);
    let top_end = body.y - lip;
    let on_lip = smoothstep(top_end - 0.5, top_end + 0.5, q.y);
    let lip_shade = mix(0.80, 0.60, clamp((q.y - top_end) / lip, 0.0, 1.0));
    // The ridge between the top and the front catches the light
    let ridge = (1.0 - smoothstep(0.0, 1.5, abs(top_end - 1.0 - q.y))) * (1.0 - travel);
    color = mix(color + vec3<f32>(0.03) * ridge, color * lip_shade, on_lip);

    return vec4<f32>(to_linear(color), 1.0) * alpha;
}

fn black_key(p: vec2<f32>, size: vec2<f32>, tint: vec4<f32>, travel: f32) -> vec4<f32> {
    let w = size.x;
    let h = size.y;

    let dist = key_shape(p, size, clamp(w * 0.10, 1.5, 9.0));
    let alpha = 1.0 - smoothstep(-0.5, 0.5, dist);

    let v = clamp(p.y / h, 0.0, 1.0);
    let tint_color = to_gamma(tint.rgb);

    // The top of the key is narrower than its base: both sides and the front slope down
    let slope = max(1.5, w * 0.11);
    let front = clamp(h * 0.09, 4.0, w * 0.8) * (1.0 - 0.55 * travel);
    let top_end = h - front;

    var top = mix(vec3<f32>(0.10, 0.10, 0.11), vec3<f32>(0.16, 0.16, 0.175), v);
    top = mix(top, tint_color, tint.a * 0.9);

    // Lacquer shines along its length
    let across = (p.x - w * 0.34) / (w * 0.09);
    let shine = exp(-across * across) * smoothstep(0.03, 0.18, v) * (1.0 - smoothstep(0.55, 0.86, v));
    top += vec3<f32>(0.09) * shine * (1.0 - 0.5 * tint.a);

    // Light comes from the left
    let lit_side = mix(vec3<f32>(0.30, 0.30, 0.32), tint_color * 1.2 + vec3<f32>(0.08), tint.a * 0.85);
    let dark_side = mix(vec3<f32>(0.035, 0.035, 0.04), tint_color * 0.4, tint.a * 0.85);
    let front_face = mix(vec3<f32>(0.22, 0.22, 0.235), tint_color * 0.72, tint.a * 0.85);

    var color = mix(lit_side, top, smoothstep(slope - 0.75, slope + 0.75, p.x));
    color = mix(color, dark_side, smoothstep(w - slope - 0.75, w - slope + 0.75, p.x));

    let on_front = smoothstep(top_end - 0.75, top_end + 0.75, p.y);
    let front_shade = mix(1.45, 0.55, clamp((p.y - top_end) / front, 0.0, 1.0));
    color = mix(color, front_face * front_shade, on_front);

    // The fallboard shades the back of the keys
    color *= 1.0 - 0.45 * exp(-p.y / (h * 0.06));
    color *= 1.0 - 0.12 * travel;

    return vec4<f32>(to_linear(color), 1.0) * alpha;
}

// The instance is larger than the key it belongs to, and centered on it
fn black_key_shadow(p: vec2<f32>, size: vec2<f32>, key_size: vec2<f32>, travel: f32) -> vec4<f32> {
    let w = key_size.x;
    let margin = (size.x - w) * 0.5;

    // A key that is down is closer to the white keys: its shadow is shorter
    let offset = vec2<f32>(w * 0.11, w * 0.06) * (1.0 - 0.6 * travel);
    let blur = w * 0.16;

    // Twice as tall as the key, so that the shadow does not fade at the back
    let q = p - vec2<f32>(margin, -key_size.y) - offset;
    let dist = rounded_box(q, vec2<f32>(w, key_size.y * 2.0), w * 0.10);

    let alpha = (1.0 - smoothstep(-blur, blur, dist)) * 0.40 * (1.0 - 0.3 * travel);
    return vec4<f32>(0.0, 0.0, 0.0, alpha);
}

fn felt(p: vec2<f32>, size: vec2<f32>) -> vec4<f32> {
    let v = clamp(p.y / size.y, 0.0, 1.0);
    let color = mix(vec3<f32>(0.56, 0.08, 0.10), vec3<f32>(0.33, 0.03, 0.05), v * v);
    return vec4<f32>(to_linear(color), 1.0);
}

fn cheek(p: vec2<f32>, size: vec2<f32>, is_right: bool) -> vec4<f32> {
    let v = clamp(p.y / size.y, 0.0, 1.0);
    var color = mix(vec3<f32>(0.105, 0.10, 0.098), vec3<f32>(0.055, 0.052, 0.05), v);

    // The edge next to the keys catches some light
    let from_keys = select(size.x - p.x, p.x, is_right);
    color += vec3<f32>(0.07) * (1.0 - smoothstep(0.0, 2.0, from_keys));
    // And so does the top edge
    color += vec3<f32>(0.04) * (1.0 - smoothstep(0.0, 1.5, p.y));

    return vec4<f32>(to_linear(color), 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let kind = in.params.x;
    let travel = in.params.y;

    if kind < WHITE_KEY + 0.5 {
        return white_key(in.local, in.size, in.tint, travel);
    } else if kind < BLACK_KEY + 0.5 {
        return black_key(in.local, in.size, in.tint, travel);
    } else if kind < BLACK_KEY_SHADOW + 0.5 {
        return black_key_shadow(in.local, in.size, in.params.zw, travel);
    }

    // The other kinds are plain boxes
    let dist = rounded_box(in.local, in.size, 0.0);
    let alpha = 1.0 - smoothstep(-0.5, 0.5, dist);

    if kind < BED + 0.5 {
        return vec4<f32>(to_linear(vec3<f32>(0.03, 0.03, 0.032)), 1.0) * alpha;
    } else if kind < FELT + 0.5 {
        return felt(in.local, in.size) * alpha;
    }

    return cheek(in.local, in.size, travel > 0.5) * alpha;
}
