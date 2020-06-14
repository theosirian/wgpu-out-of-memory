#version 450

layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec4 in_color;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in uint in_index;

layout(set = 0, binding = 0) uniform Global { mat4 camera; };
layout(set = 0, binding = 1) uniform Local { mat4 transform; };

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec2 out_uv;
layout(location = 2) out uint out_index;

void main() {
    out_color = in_color;
    out_uv = in_uv;
    out_index = in_index;

    gl_Position = camera * transform * vec4(in_pos, 0.0, 1.0);
}