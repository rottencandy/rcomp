#version 330 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 texcoord;

out vec2 Tex;

void main()
{
    Tex = texcoord;
    gl_Position = vec4(Position, 1.0, 1.0);
}
