#version 330 core
layout (location = 0) in vec2 aPos;
layout (location = 1) in vec3 aColor;

out vec3 ourColor;
out vec2 TexCoord;

uniform float scale = 1;

void main()
{
    gl_Position = vec4(aPos * scale, 0, 1);
    ourColor = aColor;
    TexCoord = 2 * aPos.xy - 1;
}