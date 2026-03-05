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
layout(std430, binding = 2) readonly buffer POD_BindPose
{
    vec4 pod_bind_pose[];
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
layout(std430, binding = 8) readonly buffer POD_Nodes_BindPose
{
    vec4 pod_nodes_bind_pose[];
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
    uint fragment_id = gl_InstanceID + 1;
    uvec4 parents = pod_parents[fragment_id];
    vec4 weights = pod_weights[fragment_id];
    vec3 bind_pose = pod_bind_pose[fragment_id].xyz;

    // common ids and weights gather
    uint i0 = imap_nodes[parents.x];
    uint i1 = imap_nodes[parents.y];
    uint i2 = imap_nodes[parents.z];
    uint i3 = imap_nodes[parents.w];
    float w0 = weights.x;
    float w1 = weights.y;
    float w2 = weights.z;
    float w3 = weights.w;

    // linear-blend-skinning for positions
    vec3 p0 = pod_nodes_positions[i0].xyz;
    vec3 p1 = pod_nodes_positions[i1].xyz;
    vec3 p2 = pod_nodes_positions[i2].xyz;
    vec3 p3 = pod_nodes_positions[i3].xyz;

    vec3 b0 = pod_nodes_bind_pose[i0].xyz;
    vec3 b1 = pod_nodes_bind_pose[i1].xyz;
    vec3 b2 = pod_nodes_bind_pose[i2].xyz;
    vec3 b3 = pod_nodes_bind_pose[i3].xyz;

    vec3 v0 = p0 + (model - b0);
    vec3 v1 = p1 + (model - b1);
    vec3 v2 = p2 + (model - b2);
    vec3 v3 = p3 + (model - b3);

    vec3 deform = w0 * v0 + w1 * v1 + w2 * v2 + w3 * v3;
    vec3 local = model + deform;

    vec4 world = vec4(local + bind_pose, 1.0);
    fs_world = world.xyz;
    fs_normal = normal;
    fs_color = vec4(vec3(0.5), 1.0);

    uint state = pod_states[fragment_id];
    gl_Position = u_projection * u_view * world * float(state);
}
