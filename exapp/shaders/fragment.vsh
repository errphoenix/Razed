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

layout(std430, binding = 0) readonly buffer POD_Parents
{
    uvec4 pod_parents[];
};
layout(std430, binding = 1) readonly buffer POD_Weights
{
    vec4 pod_weights[];
};
layout(std430, binding = 2) readonly buffer POD_Offsets
{
    vec4 pod_offsets[];
};
layout(std430, binding = 3) readonly buffer POD_States
{
    uint pod_states[];
};

layout(std430, binding = 6) readonly buffer IMap_Nodes
{
    uint imap_nodes[];
};
layout(std430, binding = 7) readonly buffer POD_Nodes_Positions
{
    // cpu physics data is vec3; padded to vec4 during upload
    vec4 pod_nodes_positions[];
};
layout(std430, binding = 8) readonly buffer POD_Nodes_Rotors
{
    vec4 pod_nodes_rotors[];
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
    vec3 model = vertex.position.xyz * 0.75;
    vec3 normal = normalize(vertex.normal.xyz);

    // account for degenerate 0
    uint fragment_id = gl_InstanceID + 1;
    uvec4 parents = pod_parents[fragment_id];
    vec4 weights = pod_weights[fragment_id];

    // common ids and weights gather
    uint i0 = imap_nodes[parents.x];
    uint i1 = imap_nodes[parents.y];
    uint i2 = imap_nodes[parents.z];
    uint i3 = imap_nodes[parents.w];
    float w0 = weights.x;
    float w1 = weights.y;
    float w2 = weights.z;
    float w3 = weights.w;

    // linear-blend-skinning for rotations
    vec4 r0 = pod_nodes_rotors[i0];
    vec4 r1 = pod_nodes_rotors[i1];
    vec4 r2 = pod_nodes_rotors[i2];
    vec4 r3 = pod_nodes_rotors[i3];

    // linear-blend-skinning for positions
    vec3 p0 = pod_nodes_positions[i0].xyz;
    vec3 p1 = pod_nodes_positions[i1].xyz;
    vec3 p2 = pod_nodes_positions[i2].xyz;
    vec3 p3 = pod_nodes_positions[i3].xyz;

    // world position before calibration
    vec3 fragment_base_position = p0 * w0 + p1 * w1 + p2 * w2 + p3 * w3;
    vec3 fragment_offset = pod_offsets[fragment_id].xyz;

    vec3 dir0 = normalize(p0 - fragment_base_position);
    vec3 dir1 = normalize(p1 - fragment_base_position);
    vec3 dir2 = normalize(p2 - fragment_base_position);
    vec3 dir3 = normalize(p3 - fragment_base_position);

    float nv0 = dot(model, dir0);
    float nv1 = dot(model, dir1);
    float nv2 = dot(model, dir2);
    float nv3 = dot(model, dir3);

    // avoid degenerate nodes' weights (todo)
    if (i0 == 0) nv0 = 0.0;
    if (i1 == 0) nv1 = 0.0;
    if (i2 == 0) nv2 = 0.0;
    if (i3 == 0) nv3 = 0.0;

    // align
    w0 += nv0;
    w1 += nv1;
    w2 += nv2;
    w3 += nv3;

    vec3 fragment_position = p0 * w0 + p1 * w1 + p2 * w2 + p3 * w3;
    vec3 local = model + fragment_position;

    vec4 world = vec4(local + fragment_offset, 1.0);
    fs_world = world.xyz;
    fs_normal = normal;
    fs_color = vec4(vec3(0.35), 1.0);

    uint state = pod_states[fragment_id];
    gl_Position = u_projection * u_view * world * float(state);
}
