#version 460 core

layout(std430, binding = 0) readonly buffer POD_Points
{
    vec4 pod_points[];
};

out vec4 fs_color;

uniform mat4 u_projection;
uniform mat4 u_view;

void main() {
    vec3 point = pod_points[gl_InstanceID].xyz;
    fs_color = vec4(1.0, 0.0, 1.0, 1.0);

    gl_Position = u_projection * u_view * vec4(point, 1.0);
}
