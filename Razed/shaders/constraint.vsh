#version 460 core

struct IndirectIndex {
    uint index;
    uint generation;
};
struct DirectIndex {
    uint index;
    uint generation;
};

struct Constraint {
    IndirectIndex node_pair[2];
};

layout(std430, binding = 4) readonly buffer POD_Constraints
{
    Constraint constraints[];
};

layout(std430, binding = 5) readonly buffer IMap_Nodes
{
    IndirectIndex imap_nodes[];
};
layout(std430, binding = 6) readonly buffer POD_Nodes
{
    vec4 pod_nodes[];
};

layout(std430, binding = 7) readonly buffer I_Selected
{
    DirectIndex i_selected;
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec4 fs_color;

void main() {
    uint constraint_id = gl_InstanceID;
    uint node_offset = gl_VertexID;

    Constraint constraint = constraints[constraint_id];
    IndirectIndex node_id = constraint.node_pair[node_offset];
    IndirectIndex node_ii = imap_nodes[node_id.index];

    fs_color = vec4(0.0, 1.0, 0.0, 0.4);
    if (constraint_id == i_selected.index) {
        fs_color = vec4(1.0, 0.0, 0.0, 1.0);
    }

    vec3 position = pod_nodes[node_ii.index].xyz;
    gl_Position = u_projection * u_view * vec4(position, 1.0);
}
