#version 460

in vec3 fs_world;
in vec3 fs_normal;
in vec4 fs_color;

out vec4 outColor;

uniform vec3 u_camera_forward;

const float LIGHT_AMBIENT = 0.1;

void main() {
    vec4 albedo = fs_color;
    vec3 normal = fs_normal;

    // basic directional light (camera source)
    vec3 light_dir = -u_camera_forward;
    float diffuse = dot(light_dir, normal);

    float light_factor = LIGHT_AMBIENT + diffuse;

    outColor = vec4(fs_color.rgb * light_factor, fs_color.a);
}
