use rendrs::geometry::DomainData;

pub fn geom_plane_pass() -> PlaneGeomPass {
    geom_plane_pass_with_shader(ComputeShaderPlaneGeomSubmit::new_compiled())
}

pub fn geom_plane_pass_with_shader(shader: ComputeShaderPlaneGeomSubmit) -> PlaneGeomPass {
    PlaneGeomPass::new(shader, [], [], |_section, shader, ctx, out| {
        let PlaneGeomCtx {
            _marker,
            size,
            uv_scaling,
        } = ctx;

        shader.uniform_size_floatv([*size]);
        shader.uniform_uv_scaling_floatv([*uv_scaling]);

        out.write(DomainData::new(0, 0, 1));
    })
}

rendrs::geometry_submission_job! {
    Plane => {
        source {
            vertex => super::VertexBuffers;
            triangle => super::TriangleBuffers;
        };

        uniform {
            length 1, size       : float => f32;
            length 1, uv_scaling : float => f32;
        }
        context {
            size       : f32;
            uv_scaling : f32;
        }

        "
        const vec3 p00 = vec3(-size, 0.0, -size);
        const vec3 p10 = vec3( size, 0.0, -size);
        const vec3 p01 = vec3(-size, 0.0,  size);
        const vec3 p11 = vec3( size, 0.0,  size);
        const vec3 N   = vec3(0.0, 1.0, 0.0);
        const vec2 u00 = vec2(0.0, 0.0);
        const vec2 u10 = vec2(1.0, 0.0) * uv_scaling;
        const vec2 u01 = vec2(0.0, 1.0) * uv_scaling;
        const vec2 u11 = vec2(1.0, 1.0) * uv_scaling;

        const uint vert_base = AllocVertex(4);
        const uint tris_base = AllocTriangle(2);

        const uint v0 = vert_base + 0;
        const uint v1 = vert_base + 1;
        const uint v2 = vert_base + 2;
        const uint v3 = vert_base + 3;

        VertexData(v0, p00, N, u00);
        VertexData(v1, p10, N, u10);
        VertexData(v2, p01, N, u01);
        VertexData(v3, p11, N, u11);

        TriangleData(tris_base + 0, uint[] (v0, v1, v2), 0, 0);
        TriangleData(tris_base + 1, uint[] (v2, v1, v3), 0, 0);

        "
    }
}
