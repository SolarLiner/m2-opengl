#version 330 core
out vec4 FragColor;

in vec3 ourColor;
in vec2 TexCoord;

uniform sampler2D texture1;
uniform sampler2D texture2;

void main()
{
    vec3 base = texture(texture1, TexCoord).rgb;
    vec4 overlay = texture(texture2, TexCoord);
    FragColor = vec4(mix(base, overlay.rgb, overlay.a), 1.0);
}
