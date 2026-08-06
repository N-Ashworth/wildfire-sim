#version 330 core

layout (location = 0) in vec3 aPos; 

uniform vec2 position;
uniform vec2 scale;
uniform vec2 cam_scale;

void main()
{
    vec3 pos = vec3((aPos.xy * scale + position) / cam_scale, aPos.z);
    gl_Position = vec4(pos, 1.0);
}