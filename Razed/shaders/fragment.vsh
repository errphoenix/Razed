#version 460 core

struct Metadata {
    uint offset;
    uint length;
};
struct Vertex {
    vec4 position;
    vec4 normal;
};

struct IndirectIndex {
    uint index;
    uint generation;
};
struct DirectIndex {
    uint index;
    uint generation;
};

layout(std430, binding = 10) readonly buffer VertexStorage
{
    Vertex vertex_storage[];
};

layout(std430, binding = 11) readonly buffer MeshMetadata {
    Metadata metadata[];
};

layout(std430, binding = 0) readonly buffer POD_Anchors
{
    IndirectIndex pod_anchors[][8];
};
layout(std430, binding = 1) readonly buffer POD_Weights
{
    vec4 pod_weights[][2];
};

layout(std430, binding = 2) readonly buffer POD_BindPose
{
    vec4 pod_bind_pose[];
};

layout(std430, binding = 3) readonly buffer POD_MeshID
{
    uint pod_mesh_id[];
};

layout(std430, binding = 6) readonly buffer IMap_Deforms
{
    IndirectIndex imap_deforms[];
};
layout(std430, binding = 7) readonly buffer POD_Deforms_Positions
{
    // cpu deform data is vec3; padded to vec4 during upload
    vec4 pod_deforms_positions[];
};
layout(std430, binding = 8) readonly buffer POD_Deforms_BindPose
{
    // cpu deform data is vec3; padded to vec4 during upload
    vec4 pod_deforms_pose[];
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec3 fs_world;
out vec3 fs_normal;
out vec4 fs_color;

void main() {
    // account for degenerate 0
    uint fragment_id = gl_InstanceID + 1;

    uint mesh_id = pod_mesh_id[fragment_id];
    Metadata metadata = metadata[mesh_id];
    uint offset = metadata.offset;
    uint index = offset + gl_VertexID;
    Vertex vertex = vertex_storage[index];
    vec3 model = vertex.position.xyz;
    vec3 normal = normalize(vertex.normal.xyz);

    IndirectIndex[8] anchors = pod_anchors[fragment_id];
    vec4[2] weights = pod_weights[fragment_id];
    vec3 bind_pose = pod_bind_pose[fragment_id].xyz;

    // common ids and weights gather
    uint i0 = imap_deforms[anchors[0].index].index;
    uint i1 = imap_deforms[anchors[1].index].index;
    uint i2 = imap_deforms[anchors[2].index].index;
    uint i3 = imap_deforms[anchors[3].index].index;
    uint i4 = imap_deforms[anchors[4].index].index;
    uint i5 = imap_deforms[anchors[5].index].index;
    uint i6 = imap_deforms[anchors[6].index].index;
    uint i7 = imap_deforms[anchors[7].index].index;

    float w0 = weights[0].x;
    float w1 = weights[0].y;
    float w2 = weights[0].z;
    float w3 = weights[0].w;
    float w4 = weights[1].x;
    float w5 = weights[1].y;
    float w6 = weights[1].z;
    float w7 = weights[1].w;

    vec3 p0 = pod_deforms_positions[i0].xyz;
    vec3 p1 = pod_deforms_positions[i1].xyz;
    vec3 p2 = pod_deforms_positions[i2].xyz;
    vec3 p3 = pod_deforms_positions[i3].xyz;
    vec3 p4 = pod_deforms_positions[i4].xyz;
    vec3 p5 = pod_deforms_positions[i5].xyz;
    vec3 p6 = pod_deforms_positions[i6].xyz;
    vec3 p7 = pod_deforms_positions[i7].xyz;

    vec3 b0 = pod_deforms_pose[i0].xyz;
    vec3 b1 = pod_deforms_pose[i1].xyz;
    vec3 b2 = pod_deforms_pose[i2].xyz;
    vec3 b3 = pod_deforms_pose[i3].xyz;
    vec3 b4 = pod_deforms_pose[i4].xyz;
    vec3 b5 = pod_deforms_pose[i5].xyz;
    vec3 b6 = pod_deforms_pose[i6].xyz;
    vec3 b7 = pod_deforms_pose[i7].xyz;

    vec3 w_rest = bind_pose + model;

    float d0 = distance(w_rest, b0) + 0.000001;
    float d1 = distance(w_rest, b1) + 0.000001;
    float d2 = distance(w_rest, b2) + 0.000001;
    float d3 = distance(w_rest, b3) + 0.000001;
    float d4 = distance(w_rest, b4) + 0.000001;
    float d5 = distance(w_rest, b5) + 0.000001;
    float d6 = distance(w_rest, b6) + 0.000001;
    float d7 = distance(w_rest, b7) + 0.000001;

    const float RIGIDITY = 4.0;
    float vw0 = 1.0 / pow(d0, RIGIDITY);
    float vw1 = 1.0 / pow(d1, RIGIDITY);
    float vw2 = 1.0 / pow(d2, RIGIDITY);
    float vw3 = 1.0 / pow(d3, RIGIDITY);
    float vw4 = 1.0 / pow(d4, RIGIDITY);
    float vw5 = 1.0 / pow(d5, RIGIDITY);
    float vw6 = 1.0 / pow(d6, RIGIDITY);
    float vw7 = 1.0 / pow(d7, RIGIDITY);

    float vwt = vw0 + vw1 + vw2 + vw3 + vw4 + vw5 + vw6 + vw7;
    vw0 /= vwt;
    vw1 /= vwt;
    vw2 /= vwt;
    vw3 /= vwt;
    vw4 /= vwt;
    vw5 /= vwt;
    vw6 /= vwt;
    vw7 /= vwt;

    vec3 deform = vec3(0.0);
    if (i0 != 0) deform += vw0 * (p0 - b0);
    if (i1 != 0) deform += vw1 * (p1 - b1);
    if (i2 != 0) deform += vw2 * (p2 - b2);
    if (i3 != 0) deform += vw3 * (p3 - b3);
    if (i4 != 0) deform += vw4 * (p4 - b4);
    if (i5 != 0) deform += vw5 * (p5 - b5);
    if (i6 != 0) deform += vw6 * (p6 - b6);
    if (i7 != 0) deform += vw7 * (p7 - b7);

    vec4 world = vec4(deform + w_rest, 1.0);
    fs_world = world.xyz;
    fs_normal = mix(normal, normalize(abs(world.xyz)), 0.35);
    fs_color = vec4(vec3(0.8), 1.0);

    gl_Position = u_projection * u_view * world;
}
