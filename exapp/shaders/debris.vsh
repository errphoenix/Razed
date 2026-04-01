#version 460 core

struct Metadata {
    uint offset;
    uint length;
};

struct Vertex {
    vec4 position;
    vec4 normal;
};

layout(std430, binding = 10) readonly buffer VertexStorage
{
    Vertex vertex_storage[];
};

layout(std430, binding = 11) readonly buffer MeshMetadata {
    Metadata metadata[];
};

layout(std430, binding = 0) readonly buffer POD_Positions
{
    vec4 pod_positions[];
};
layout(std430, binding = 1) readonly buffer POD_Rotations
{
    vec4 pod_rotations[];
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec3 fs_world;
out vec3 fs_normal;
out vec4 fs_color;

// debug cube
const uint MESH_ID = 0;

void main() {
    Metadata metadata = metadata[MESH_ID];
    uint offset = metadata.offset;
    uint index = offset + gl_VertexID;

    Vertex vertex = vertex_storage[index];
    vec3 model = vertex.position.xyz;
    vec3 normal = normalize(vertex.normal.xyz);

    // account for degenerate 0
    uint debris_id = gl_InstanceID + 1;
    vec3 position = pod_positions[debris_id].xyz;
    //vec4 rotation = pod_rotations[debris_id];

    vec4 world = vec4(position, 1.0);
    fs_world = world.xyz;
    fs_normal = normal;
    fs_color = vec4(vec3(0.8), 1.0);

    //uint state = pod_states[fragment_id];
    //gl_Position = u_projection * u_view * world * float(state);
    gl_Position = u_projection * u_view * world;
}
