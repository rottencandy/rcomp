#version 330 core

in vec2 Tex;

uniform sampler2D texImage;

void main()
{
    //gl_FragColor = vec4(0.6, 0.6, 0.8, 1.0);
    gl_FragColor = texture(texImage, Tex);
}
