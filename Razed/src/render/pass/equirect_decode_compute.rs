pub const WORKGROUP_SIZE_XY: u32 = 8;

ethel::shader_glsl_compute! {
    struct EquirectDecode > [460] {
        workgroup [8, 8, 1];

        uniform {
            length 1, resolution_src : uvec2 = (u32, u32);
            length 1, resolution_face: uvec2 = (u32, u32);

            length 1, in_equirect: image2D => i32;
            length 1, out_cubemap: imageCube => i32;
        };

        src() {
            "
            uint x = gl_GlobalInvocationID.x;
            uint y = gl_GlobalInvocationID.y;
            uint face = gl_GlobalInvocationID.z;

            vec2 uv_n = (vec2(x, y) + 0.5) / vec2(resolution_face);
            float u = uv_n.x * 2.0 - 1.0;
            float v = uv_n.y * 2.0 - 1.0;

            vec3 dir;
            switch(face) {
                case 0:
                    dir = vec3( 1.0, -v, -u);
                    break;
                case 1:
                    dir = vec3(-1.0, -v,  u);
                    break;
                case 2:
                    dir = vec3( u,  1.0,  v);
                    break;
                case 3:
                    dir = vec3( u, -1.0, -v);
                    break;
                case 4:
                    dir = vec3( u, -v,  1.0);
                    break;
                case 5:
                    dir = vec3(-u, -v, -1.0);
                    break;
            }

            const float PI = 3.14159;
            float phi = atan(dir.z, dir.x);
            float rho = acos(dir.y);
            vec2 uv = vec2((phi / (2.0*PI))+0.5, rho / PI);
            ivec2 px = uv * float(resolution_src + 1);

            vec4 C = imageLoad(in_equirect, px);
            imageStore(out_cubemap, ivec3(x, y, face), C);
            ";
        }
    }
}
