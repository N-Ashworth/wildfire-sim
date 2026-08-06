#version 330 core

// Output color to the screen framebuffer
out vec4 FragColor;

uniform vec3 color;

void main()
{
    FragColor = vec4(color, 1.0);
}
