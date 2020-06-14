#version 450

layout(location = 0) in vec4 in_color;
layout(location = 1) in vec2 in_uv;
layout(location = 2) flat in uint in_index;

layout(set = 1, binding = 0) uniform texture2D u_texture;
layout(set = 1, binding = 1) uniform sampler u_sampler;

layout(location = 0) out vec4 o_color;

void main() {
    o_color = texture(sampler2D(u_texture, u_sampler), in_uv);
}