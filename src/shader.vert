#version 330 core

layout (location = 0) in vec3 aPos; 

uniform vec2 position;
uniform vec2 scale;
uniform vec2 cam_scale;
uniform vec2 cam_pos;

void main()
{
    vec2 world = aPos.xy * scale + position;
    gl_Position = vec4((world - cam_pos) / cam_scale, aPos.z, 1.0);
}