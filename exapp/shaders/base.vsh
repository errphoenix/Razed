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

struct Entity {
    uint mesh_index;
    uint position_id;
    uint rotation_id;

    // Pad to 16 bytes for compatibility
    // While on my system the ssbo alignment is 4, for now we will force 
    // alignment to 16 bytes to ensure compatibility. we might handle this 
    // differently later on.
    uint _pad; 
};

layout(std430, binding = 0) readonly buffer EntityIndexMap 
{
    Entity entities[];
};

layout(std430, binding = 1) readonly buffer MeshData 
{
    uint mesh_ids[];
};

layout(std430, binding = 2) readonly buffer IMap_Positions
{
    uint imap_positions[];
};
layout(std430, binding = 3) readonly buffer IMap_Rotations
{
    uint imap_rotations[];
};

layout(std430, binding = 4) readonly buffer POD_Positions
{
    vec4 pod_positions[]; 
};
layout(std430, binding = 5) readonly buffer POD_Rotations
{
    vec4 pod_rotations[];
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec3 fs_world;
out vec3 fs_normal;

vec4 mulQuat(vec4 q0, vec4 q1);

vec3 rotateQuat(vec3 p, vec4 q) {
    vec4 q_conj = vec4(-q.x, -q.y, -q.z, q.w);
    vec4 p4 = vec4(p, 1.0);

    vec4 r = mulQuat(q, p4);
    r = mulQuat(r, q_conj);
    return r.xyz;
}

void main() {
    Entity mapping = entities[gl_DrawID];
    uint mesh_id_index = mapping.mesh_index;
    uint position_index = imap_positions[mapping.position_id];
    uint rotation_index = imap_rotations[mapping.rotation_id];

    uint mesh_id = mesh_ids[mesh_id_index];
    vec3 position = pod_positions[position_index].xyz;
    vec4 rotation = pod_rotations[rotation_index];

    Metadata metadata = metadata[mesh_id];
    uint offset = metadata.offset;
    uint index = offset + gl_VertexID;

    Vertex vertex = vertex_storage[index];
    vec3 model = vertex.position.xyz;
    vec3 normal = vertex.normal.xyz;

    vec3 local = rotateQuat(model, rotation);
    vec4 world = vec4(local + position, 1.0);

    fs_world = world.xyz;
    fs_normal = normal;
    
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