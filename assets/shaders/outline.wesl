// Draws a one-texel outline around the opaque pixels of a sprite.
// - The vertex shader grows the sprite's quad by one texel on each side.
// - A frag is part of the outline if it's transparent itself but has an opaque
//   neighbour, w/ everything else discarded.

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var image: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var image_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> color: vec4<f32>;
// x: grow upwards (0 or 1), y: grow downwards
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> grow: vec4<f32>;

// Bound in OutlineMaterial::specialize.
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) rect: vec4<f32>,
    @location(3) uv_rect: vec4<f32>,
}

struct OutlineOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) uv_rect: vec4<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> OutlineOutput {
    let texel = 1.0 / vec2<f32>(textureDimensions(image));

    // which edges this vertex is on, in the quad and in the image
    let s = sign(vertex.position.xy - 0.5 * (vertex.rect.xy + vertex.rect.zw));
    let t = sign(vertex.uv - 0.5 * (vertex.uv_rect.xy + vertex.uv_rect.zw));
    let grow_y = select(grow.y, grow.x, s.y > 0.0);

    // the quad's size over the region's size in texels
    let texel_world = (vertex.rect.zw - vertex.rect.xy) * texel / (vertex.uv_rect.zw - vertex.uv_rect.xy);
    let position = vertex.position + vec3(s.x * texel_world.x, s.y * grow_y * texel_world.y, 0.0);
    let uv = vertex.uv + vec2(t.x * texel.x, t.y * grow_y * texel.y);

    var out: OutlineOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(position, 1.0));
    out.position = position_world_to_clip(world_position.xyz);
    out.uv = uv;
    out.uv_rect = vertex.uv_rect;
    return out;
}

fn alpha(uv: vec2<f32>, rect: vec4<f32>) -> f32 {
    if any(uv < rect.xy) || any(uv > rect.zw) {
        return 0.0;
    }
    return textureSampleLevel(image, image_sampler, uv, 0.0).a;
}

@fragment
fn fragment(in: OutlineOutput) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(image));
    let here = alpha(in.uv, in.uv_rect);
    let neighbours = max(max(alpha(in.uv + vec2(texel.x, 0.0), in.uv_rect), alpha(in.uv - vec2(texel.x, 0.0), in.uv_rect)),
                         max(alpha(in.uv + vec2(0.0, texel.y), in.uv_rect), alpha(in.uv - vec2(0.0, texel.y), in.uv_rect)));
    if here >= 0.5 || neighbours < 0.5 {
        discard;
    }
    return color;
}
