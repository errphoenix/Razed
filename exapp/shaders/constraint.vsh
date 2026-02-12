#version 460 core

struct Constraint {
    uint node_pair[2];
};

layout(std430, binding = 4) readonly buffer POD_Constraints
{
    Constraint constraints[];
};

layout(std430, binding = 5) readonly buffer IMap_Nodes {
    uint imap_nodes[];
};
layout(std430, binding = 6) readonly buffer POD_Nodes {
    vec4 pod_nodes[];
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec3 fs_normal;

void main() {
    uint constraint_id = gl_InstanceID;
    uint node_offset = gl_VertexID;

    Constraint constraint = constraints[constraint_id];
    uint node_id = constraint.node_pair[node_offset];
    uint node_ii = imap_nodes[node_id];

    vec3 position = pod_nodes[node_ii].xyz;

    fs_normal = vec3(0.0, 1.0, 0.0);

    gl_Position = u_projection * u_view * vec4(position, 1.0);
}
