use ethel::shader::{GlslStruct, GlslUniform, ShaderKind, ShaderProgram};
use ethel::state::data::IndirectIndex;

use crate::render::shader_commons;

macro_rules! ssbo_binding {
    (POD_Constraints) => {
        4
    };
    (IMap_Nodes) => {
        5
    };
    (POD_Nodes) => {
        6
    };
    (I_Selected) => {
        7
    };
}

ethel::shader_glsl_struct! {
    struct Constraint {
        nodes[2]: [IndirectIndex; 2] => IndirectIndex;
    }
}

pub const TYPE_CONSTRAINT: GlslStruct = ConstraintGlslStruct::as_definition();

pub const SSBO_INDEX_POD_CONSTRAINTS: u32 = ssbo_binding!(POD_Constraints);
pub const SSBO_INDEX_POD_NODES: u32 = ssbo_binding!(POD_Nodes);
pub const SSBO_INDEX_I_SELECTED: u32 = ssbo_binding!(I_Selected);
pub const SSBO_INDEX_IMAP_NODES: u32 = ssbo_binding!(IMap_Nodes);

ethel::shader_glsl! {
    struct DebugLattice > [460] {
        common {};

        unit ShaderKind::Vertex => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output fs_color: vec4;
                }
            };

            uniform {
                length 1, projection: mat4 => glam::Mat4;
                length 1, view: mat4 => glam::Mat4;
            };

            type {
                shader_commons::TYPE_INDEX_INDIRECT
                shader_commons::TYPE_INDEX_DIRECT

                TYPE_CONSTRAINT
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Constraints => {
                        [dyn_array Constraint: constraints]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf IMap_Nodes => {
                        [dyn_array IndirectIndex: imap_nodes]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Nodes => {
                        [dyn_array vec4: pod_nodes]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf I_Selected => {
                        DirectIndex: i_selected;
                    }
                }
            };

            src() "
                uint constraint_id = gl_InstanceID;
                uint node_offset = gl_VertexID;

                Constraint constraint = constraints[constraint_id];
                IndirectIndex node_id = constraint.nodes[node_offset];
                IndirectIndex node_ii = imap_nodes[node_id.index];

                fs_color = vec4(0.0, 1.0, 0.0, 0.28);
                if (constraint_id == i_selected.index) {
                    fs_color = vec4(1.0, 0.0, 0.0, 1.0);
                }

                vec3 position = pod_nodes[node_ii.index].xyz;
                gl_Position = projection * view * vec4(position, 1.0);
            "
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    input fs_color: vec4;
                    output out_Color: vec4;
                }
            };

            src() "
                out_Color = fs_color;
            "
        ];
    }
}
