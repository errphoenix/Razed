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

vec4 mulQuat(vec4 q0, vec4 q1);

vec3 rotateQuat(vec3 p, vec4 q) {
    vec4 q_conj = vec4(-q.x, -q.y, -q.z, q.w);
    vec4 p4 = vec4(p, 1.0);

    vec4 r = mulQuat(q, p4);
    r = mulQuat(r, q_conj);
    return r.xyz;
}

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

    vec3 local = model;
    vec4 rotation_lbs = normalize((r0 * w0) + (r1 * w1) + (r2 * w2) + (r3 * w3));
    local = rotateQuat(local, rotation_lbs);

    // linear-blend-skinning for positions
    vec3 p0 = pod_nodes_positions[i0].xyz;
    vec3 p1 = pod_nodes_positions[i1].xyz;
    vec3 p2 = pod_nodes_positions[i2].xyz;
    vec3 p3 = pod_nodes_positions[i3].xyz;

    vec3 fragment_base = pod_offsets[fragment_id].xyz;
    vec3 fragment_offset = p0 * w0 + p1 * w1 + p2 * w2 + p3 * w3;
    vec3 fragment_pos = fragment_base + fragment_offset;

    vec4 world = vec4(local + fragment_pos, 1.0);
    fs_world = world.xyz;
    fs_normal = rotateQuat(normal, rotation_lbs);
    fs_color = vec4(vec3(0.35), 1.0);

    gl_Position = u_projection * u_view * world;
}

vec4 mulQuat(vec4 q0, vec4 q1) {
    vec4 r;
    r.x = (q0.w * q1.x) + (q0.x * q1.w) + (q0.y * q1.z) - (q0.z * q1.y);
    r.y = (q0.w * q1.y) - (q0.x * q1.z) + (q0.y * q1.w) + (q0.z * q1.x);
    r.z = (q0.w * q1.z) + (q0.x * q1.y) - (q0.y * q1.x) + (q0.z * q1.w);
    r.w = (q0.w * q1.w) - (q0.x * q1.x) - (q0.y * q1.y) - (q0.z * q1.z);
    return r;
}
