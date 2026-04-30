use super::commons;
use ethel::shader::{GlslUniform, ShaderKind};

mod lattice {
    use ethel::{shader::GlslStruct, state::data::IndirectIndex};

    ethel::shader_glsl_struct! {
        struct Constraint {
            nodes[2]: [IndirectIndex; 2] => IndirectIndex;
        }
    }

    pub const TYPE_CONSTRAINT: GlslStruct = ConstraintGlslStruct::as_definition();
}

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
                projection: mat4 => glam::Mat4;
                view: mat4 => glam::Mat4;
            };

            type {
                commons::TYPE_INDEX_INDIRECT
                commons::TYPE_INDEX_DIRECT

                lattice::TYPE_CONSTRAINT
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Constraints on 4 => {
                        [dyn_array Constraint: constraints]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf IMap_Nodes on 5 => {
                        [dyn_array IndirectIndex: imap_nodes]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Nodes on 6 => {
                        [dyn_array vec4: pod_nodes]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf I_Selected on 7 => {
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
