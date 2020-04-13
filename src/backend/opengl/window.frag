#version 330 core

in vec2 Tex;

uniform sampler2D texImage;

void main()
{
    gl_FragColor = texture(texImage, Tex);
}
