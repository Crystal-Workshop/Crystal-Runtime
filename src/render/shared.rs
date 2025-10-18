pub(crate) const SHADER: &str = r#"
struct GlobalUniform {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_position: vec4<f32>,
    light_color: vec4<f32>,
}

struct ObjectConstants {
    model: mat4x4<f32>,
    normal: mat3x4<f32>,
    color: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> globals: GlobalUniform;

@group(1) @binding(0)
var<uniform> object: ObjectConstants;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = object.model * vec4<f32>(input.position, 1.0);
    output.position = globals.view_proj * world_pos;
    output.normal = normalize((object.normal * vec4<f32>(input.normal, 0.0)).xyz);
    output.world_pos = world_pos.xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(globals.light_position.xyz - input.world_pos);
    let normal = normalize(input.normal);
    let diffuse = max(dot(normal, light_dir), 0.0);
    let ambient = 0.15;
    let intensity = globals.light_color.w;
    let light_color = globals.light_color.xyz;
    let lit_color = (ambient + diffuse * intensity) * object.color.rgb * light_color;
    return vec4<f32>(lit_color, object.color.a);
}
"#;

pub(crate) const DEFAULT_CUBE_VERTICES: &[f32] = &[
    // positions        // normals
    -0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.5, 0.5, 0.5, 0.0, 0.0, 1.0,
    -0.5, 0.5, 0.5, 0.0, 0.0, 1.0, -0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.5, -0.5, -0.5, 0.0, 0.0,
    -1.0, 0.5, 0.5, -0.5, 0.0, 0.0, -1.0, -0.5, 0.5, -0.5, 0.0, 0.0, -1.0, -0.5, -0.5, -0.5, -1.0,
    0.0, 0.0, -0.5, -0.5, 0.5, -1.0, 0.0, 0.0, -0.5, 0.5, 0.5, -1.0, 0.0, 0.0, -0.5, 0.5, -0.5,
    -1.0, 0.0, 0.0, 0.5, -0.5, -0.5, 1.0, 0.0, 0.0, 0.5, -0.5, 0.5, 1.0, 0.0, 0.0, 0.5, 0.5, 0.5,
    1.0, 0.0, 0.0, 0.5, 0.5, -0.5, 1.0, 0.0, 0.0, -0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.5, -0.5,
    -0.5, 0.0, -1.0, 0.0, 0.5, -0.5, 0.5, 0.0, -1.0, 0.0, -0.5, -0.5, 0.5, 0.0, -1.0, 0.0, -0.5,
    0.5, -0.5, 0.0, 1.0, 0.0, 0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.5, 0.5, 0.5, 0.0, 1.0, 0.0, -0.5,
    0.5, 0.5, 0.0, 1.0, 0.0,
];

pub(crate) const DEFAULT_CUBE_INDICES: &[u32] = &[
    0, 1, 2, 0, 2, 3, // front
    4, 6, 5, 4, 7, 6, // back
    8, 9, 10, 8, 10, 11, // left
    12, 14, 13, 12, 15, 14, // right
    16, 18, 17, 16, 19, 18, // bottom
    20, 21, 22, 20, 22, 23, // top
];
