#version 460 core

struct ControlPoint {
    uint id;
    float weight;
};

layout(std430, binding = 0) readonly buffer POD_Points
{
    vec4 pod_points[];
};
layout(std430, binding = 1) readonly buffer POD_Controls
{
    ControlPoint pod_controls[][8];
};

layout(std430, binding = 2) readonly buffer IMap_Nodes
{
    uint imap_nodes[];
};
layout(std430, binding = 3) readonly buffer POD_Nodes_Positions
{
    vec4 pod_nodes_positions[];
};

out vec4 fs_color;

uniform mat4 u_projection;
uniform mat4 u_view;

const uint CONTROL_POINTS_COUNT = 8;

void main() {
    // account for degenerate 0
    uint global_id = gl_InstanceID + 1;

    uint deform_id = global_id / CONTROL_POINTS_COUNT;
    uint local_id = global_id % CONTROL_POINTS_COUNT;

    uint end = gl_VertexID; // 0 = deform; 1 = node

    vec3 deform_point = pod_points[deform_id].xyz;

    ControlPoint control_point = pod_controls[deform_id][local_id];
    uint node_id = imap_nodes[control_point.id];
    vec3 node_pos = pod_nodes_positions[node_id].xyz;

    vec3 point = mix(deform_point, node_pos, float(end));
    // force to NaN (discard) if degenerate node ID
    point /= min(1.0, float(node_id));

    fs_color = vec4(0.0, 0.0, 1.0, 1.0);

    gl_Position = u_projection * u_view * vec4(point, 1.0);
}
