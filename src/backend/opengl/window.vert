#version 330 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 texcoord;
uniform vec2 screenDim;

out vec2 Tex;

void main()
{
    // From pixels to 0,1
    vec2 Pos = Position.xy / screenDim.xy;

    // Flip y so 0 is on top
    Pos.y = (1.0 - Pos.y);

    // Map to NDC -1,1
    Pos.xy = Pos.xy * 2.0 - 1.0;

    Tex = texcoord;
    gl_Position = vec4(Pos, 1.0, 1.0);
}
