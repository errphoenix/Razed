#version 460

uniform mat4 u_view;
uniform mat4 u_projection;

out vec4 fs_color;

void main() {
    vec3 local = vec3(0.0);
    vec3 rgb = vec3(0.0);

    if (gl_VertexID == 0) {
        rgb = vec3(1.0, 0.0, 0.0);
    } else if (gl_VertexID == 1) {
        local = vec3(1.0, 0.0, 0.0);
        rgb = vec3(1.0, 0.0, 0.0);
    } else if (gl_VertexID == 2) {
        rgb = vec3(0.0, 1.0, 0.0);
    } else if (gl_VertexID == 3) {
        local = vec3(0.0, 1.0, 0.0);
        rgb = vec3(0.0, 1.0, 0.0);
    } else if (gl_VertexID == 4) {
        rgb = vec3(0.0, 0.0, 1.0);
    } else if (gl_VertexID == 5) {
        local = vec3(0.0, 0.0, 1.0);
        rgb = vec3(0.0, 0.0, 1.0);
    }

    fs_color = vec4(rgb, 1.0);
    gl_Position = u_projection * u_view * vec4(local, 1.0);
}
