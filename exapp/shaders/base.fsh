#version 460

in vec3 fs_world;
in vec3 fs_normal;

out vec4 outColor;

void main() {
    outColor = vec4(fs_world, 1.0);
}
