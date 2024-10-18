// Vertex shader

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: u32,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.color = model.color;

    return out;
}

// Fragment shader

var<private> COLORS: array<vec4<f32>, 16> = array(
    vec4(1.0, 0.0, 0.0, 1.0),  // Red
    vec4(0.0, 1.0, 0.0, 1.0),  // Green
    vec4(0.0, 0.0, 1.0, 1.0),  // Blue
    vec4(1.0, 1.0, 0.0, 1.0),  // Yellow
    vec4(1.0, 0.0, 1.0, 1.0),  // Magenta
    vec4(0.0, 1.0, 1.0, 1.0),  // Cyan
    vec4(0.5, 0.0, 0.0, 1.0),  // Dark Red
    vec4(0.0, 0.5, 0.0, 1.0),  // Dark Green
    vec4(0.0, 0.0, 0.5, 1.0),  // Dark Blue
    vec4(1.0, 0.5, 0.0, 1.0),  // Orange
    vec4(0.5, 0.0, 0.5, 1.0),  // Purple
    vec4(0.0, 0.5, 0.5, 1.0),  // Teal
    vec4(0.8, 0.4, 0.0, 1.0),  // Dark Orange
    vec4(0.0, 0.4, 0.8, 1.0),  // Sky Blue
    vec4(0.4, 0.8, 0.0, 1.0),  // Lime
    vec4(0.8, 0.0, 0.4, 1.0),  // Rose
);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return COLORS[in.color];
}