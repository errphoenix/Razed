use ethel::shader::Constant;
use rendrs::{
    ComputePass,
    graphics::{PixelResolution, ShCoeffsBuffer},
    pipeline::{ImageAccessKind, ImageObject, ImageObjectTarget, Sampler, SamplerObject},
};

use crate::render::{ViewData, geometry::GeometryBank};

pub type ShadePbrPass = ComputePass<ShadePbrCtxWrapper, 4, 4>;

pub const fn shade_pbr_pass(
    shader: &ComputeShaderShadePbr,
    texmap: SamplerObject,
    env_brdf: SamplerObject,
    env_dbgmap: SamplerObject,
    depth_buffer: SamplerObject,
    raster_in: ImageObject,
    shade_out: ImageObject,
    attr_frame_in: ImageObject,
    attr_grads_in: ImageObject,
) -> ShadePbrPass {
    let handle_view = shader.compute_handle().view();

    let texmap = Sampler::wrap(texmap, SAMPLER_UNIT_TEXTURE_MAP);
    let env_brdf = Sampler::wrap(env_brdf, SAMPLER_UNIT_BAKED_BRDF_SPEC);
    let env_dbgmap = Sampler::wrap(env_dbgmap, SAMPLER_UNIT_DEBUG_REFLECTION_PROBE);
    let depth_buffer = Sampler::wrap(depth_buffer, SAMPLER_UNIT_DEPTH_BUFFER);

    let raster_in = ImageObjectTarget::new(
        raster_in,
        ImageAccessKind::ReadOnly,
        IMAGE_BIND_RASTER_IN,
        None,
    );
    let shade_out = ImageObjectTarget::new(
        shade_out,
        ImageAccessKind::WriteOnly,
        IMAGE_BIND_SHADE_OUT,
        None,
    );
    let attr_frame_in = ImageObjectTarget::new(
        attr_frame_in,
        ImageAccessKind::ReadOnly,
        IMAGE_BIND_ATTR_FRAME_IN,
        None,
    );
    let attr_grads_in = ImageObjectTarget::new(
        attr_grads_in,
        ImageAccessKind::ReadOnly,
        IMAGE_BIND_ATTR_GRADS_IN,
        None,
    );
    ShadePbrPass::new(
        handle_view,
        [texmap, env_brdf, env_dbgmap, depth_buffer],
        [raster_in, shade_out, attr_frame_in, attr_grads_in],
        |_, ctx| {
            let ShadePbrCtx {
                shader,
                gbank,
                irradiance_sh,
                resolution,
                view_data,
                dev_mat_page,
            } = ctx;

            gbank
                .vertex_buffers()
                .bind_ssbo_normals(Some(SSBO_BIND_GEOM_VERTEX_NORMALS));
            gbank
                .vertex_buffers()
                .bind_ssbo_uvs(Some(SSBO_BIND_GEOM_VERTEX_UVS));
            gbank
                .triangle_buffers()
                .bind_ssbo_indices(Some(SSBO_BIND_GEOM_TRIANGLE_INDICES));
            gbank
                .triangle_buffers()
                .bind_ssbo_attribs(Some(SSBO_BIND_GEOM_TRIANGLE_ATTRIBS));
            irradiance_sh.bind_shader_storage(SSBO_BIND_PROBE_IRRADIANCE, 0);

            let m_vp = view_data.proj_mat * view_data.view_mat;
            shader.uniform_inv_viewproj_mat4v([m_vp.inverse()]);
            shader.uniform_resolution_uvec2v([[resolution.width(), resolution.height()]]);
            shader.uniform_camera_position_vec3v([view_data.view_pos]);
            shader.uniform_camera_forward_vec3v([view_data.view_dir]);
            shader.uniform_dev_material_pages_uintv(*dev_mat_page);

            let wg_x = resolution.width().div_ceil(WORKGROUP_SIZE_XY);
            let wg_y = resolution.height().div_ceil(WORKGROUP_SIZE_XY);
            [wg_x, wg_y, 1]
        },
    )
}

#[derive(Debug)]
pub struct ShadePbrCtx<'ctx> {
    pub shader: &'ctx ComputeShaderShadePbr,

    pub gbank: &'ctx GeometryBank,
    pub irradiance_sh: &'ctx ShCoeffsBuffer,

    pub resolution: PixelResolution,
    pub view_data: ViewData,

    // 0 = diffuse + alpha
    // 1 = normal + emissive
    // 2 = ormd
    pub dev_mat_page: [u32; 3],
}
rendrs::context_wrapper!(for<'ctx> ShadePbrCtx);

pub const IMAGE_BIND_RASTER_IN: u32 = 10;
pub const IMAGE_BIND_SHADE_OUT: u32 = 11;
pub const IMAGE_BIND_ATTR_FRAME_IN: u32 = 12;
pub const IMAGE_BIND_ATTR_GRADS_IN: u32 = 13;

pub const SAMPLER_UNIT_TEXTURE_MAP: u32 = 5;
pub const SAMPLER_UNIT_BAKED_BRDF_SPEC: u32 = 6;
pub const SAMPLER_UNIT_DEBUG_REFLECTION_PROBE: u32 = 7;
pub const SAMPLER_UNIT_DEPTH_BUFFER: u32 = 8;

pub const WORKGROUP_SIZE_XY: u32 = 8;

macro_rules! ssbo_binding {
    (Geometry_VertexNormals) => {
        0
    };
    (Geometry_VertexUvs) => {
        1
    };
    (Geometry_TriangleIndices) => {
        2
    };
    (Geometry_TriangleAttribs) => {
        3
    };
    (Probe_Irradiance) => {
        4
    };
}

pub const SSBO_BIND_GEOM_VERTEX_NORMALS: u32 = ssbo_binding!(Geometry_VertexNormals);
pub const SSBO_BIND_GEOM_VERTEX_UVS: u32 = ssbo_binding!(Geometry_VertexUvs);
pub const SSBO_BIND_GEOM_TRIANGLE_INDICES: u32 = ssbo_binding!(Geometry_TriangleIndices);
pub const SSBO_BIND_GEOM_TRIANGLE_ATTRIBS: u32 = ssbo_binding!(Geometry_TriangleAttribs);
pub const SSBO_BIND_PROBE_IRRADIANCE: u32 = ssbo_binding!(Probe_Irradiance);

ethel::shader_glsl_compute! {
    struct ShadePbr > [460] {
        workgroup [8, 8, 1];

        uniform {
            length 1, resolution: uvec2 => [u32; 2];

            length 1, inv_viewproj: mat4 => glam::Mat4;

            length 1, camera_forward: vec3 => glam::Vec3;
            length 1, camera_position: vec3 => glam::Vec3;

            // 0 = diffuse + alpha
            // 1 = normal + emissive
            // 2 = ormd
            length 3, dev_material_pages: uint => u32;
        };
        sampler {
            on SAMPLER_UNIT_TEXTURE_MAP, for 1     => texture_map          : sampler2DArray;
            on SAMPLER_UNIT_BAKED_BRDF_SPEC        => baked_brdf_spec      : sampler2D;
            on SAMPLER_UNIT_DEBUG_REFLECTION_PROBE => debug_reflection_env : samplerCube;
            on SAMPLER_UNIT_DEPTH_BUFFER           => depth_buffer         : sampler2D;
        };
        image {
            on IMAGE_BIND_RASTER_IN => raster_in : uimage2D as rg32ui readonly;

            on IMAGE_BIND_ATTR_FRAME_IN => attr_frame_in : image2D as rgba16  readonly;
            on IMAGE_BIND_ATTR_GRADS_IN => attr_grads_in : image2D as rgba16f readonly;

            on IMAGE_BIND_SHADE_OUT => shade_out : image2D as rgba16f writeonly;

        };
        type {
            rendrs::geometry::TYPE_TRIANGLE_ATTRIBS
            rendrs::graphics::TYPE_FRESNEL_PARAMS
            rendrs::graphics::material::shader::TYPE_MATERIAL_ENTRY_LOCATION
            rendrs::graphics::material::shader::TYPE_MATERIAL_LOCATION
            rendrs::graphics::irradiance_harmonics::TYPE_SH_COEFFS
        };
        ssbo {
            ethel::shader_glsl_ssbo! {
                buf Geometry_VertexNormals => {
                    [dyn_array float : geometry_vertex_normals => each 2]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Geometry_VertexUvs => {
                    [dyn_array float : geometry_vertex_uvs => each 2]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Geometry_TriangleIndices => {
                    [dyn_array uint : geometry_triangle_indices => each 3]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Geometry_TriangleAttribs => {
                    [dyn_array TriangleAttribs : geometry_triangle_attribs]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Probe_Irradiance => {
                    ShCoeffs : probe_irradiance;
                }
            }
        };
        const {
            crate::render::shader_commons::CONST_AMBIENT_LIGHT
            crate::render::shader_commons::CONST_REFLECTION_MAX_LOD
            Constant::new("DEV_MATERIAL_GROUP", 0u32)
            Constant::new("UV_SCALE", 0.25)
        };
        lib {
            rendrs::pack::PACK_OCTAHEDRON_DECODE;
            rendrs::geometry::rasterize::LIB_UTIL_DEPTH_WORLDPOS;
            rendrs::geometry::rasterize::LIB_UTIL_FRAMESPACE_GET_TANFRAME;
            rendrs::geometry::rasterize::LIB_UTIL_FRAMESPACE_GET_BWEIGHTS;

            rendrs::graphics::irradiance_harmonics::LIB_EVALUATE_SH_L2;
            rendrs::graphics::light::LIB_LIGHT_ATTENUATE_ISQ_WINDOWED_CURVE;

            rendrs::graphics::LIB_FRESNEL_PARAMS;
            rendrs::graphics::LIB_FRESNEL_SCHLICK;
            rendrs::graphics::LIB_NDF_GGX;
            rendrs::graphics::LIB_NDF_GGX_LAMBDA;
            rendrs::graphics::LIB_NDF_LAMBDA_A_NOSQRT;
            rendrs::graphics::LIB_NDF_MASK_SMITH_G2_HEIGHT_GGX_HAMMON_APPROX;
        };

        src() {
            "
            ivec2 id = ivec2(gl_GlobalInvocationID.xy);
            if (id.x >= resolution.x || id.y >= resolution.y) {
                return;
            }

            float depth = texelFetch(depth_buffer, id, 0).r;

            uvec2 G = imageLoad(raster_in, id).rg;   // geom data
            vec4  d = imageLoad(attr_grads_in, id);  // derivatives
            vec4  F = imageLoad(attr_frame_in, id);  // frame data

            if (G.x == 0) {
                return;
            }

            uint[3] I = geometry_triangle_indices[G.x - 1];

            float[2] UV0 = geometry_vertex_uvs[I[0]];
            float[2] UV1 = geometry_vertex_uvs[I[1]];
            float[2] UV2 = geometry_vertex_uvs[I[2]];

            float[2] NE0 = geometry_vertex_normals[I[0]];
            float[2] NE1 = geometry_vertex_normals[I[1]];
            float[2] NE2 = geometry_vertex_normals[I[2]];

            vec3 W = rendrs_FrameSpace_GetBWeights(F);
            vec2 UV = vec2(UV0[0], UV0[1]) * W.x
                    + vec2(UV1[0], UV1[1]) * W.y
                    + vec2(UV2[0], UV2[1]) * W.z;
            //UV *= UV_SCALE;

            vec2 sUv = vec2(id) + 0.5 / vec2(resolution);
            vec3 P = rendrs_DepthWorldPosition(depth, sUv, inv_viewproj);

            uint DIFFUSE_ALPHA_PAGE = dev_material_pages[0];
            uint NORMAL_EMISSIVE_PAGE = dev_material_pages[1];
            uint ORMD_PAGE = dev_material_pages[2];

            vec4 qDiffuseAlpha = textureGrad(
                texture_map[DEV_MATERIAL_GROUP],
                vec3(UV, float(DIFFUSE_ALPHA_PAGE)),
                vec2(d.x, d.y), vec2(d.z, d.w)
            );
            vec4 qNormalEmissive = textureGrad(
                texture_map[DEV_MATERIAL_GROUP],
                vec3(UV, float(NORMAL_EMISSIVE_PAGE)),
                vec2(d.x, d.y), vec2(d.z, d.w)
            );
            vec4 qOrmd = textureGrad(
                texture_map[DEV_MATERIAL_GROUP],
                vec3(UV, float(ORMD_PAGE)),
                vec2(d.x, d.y), vec2(d.z, d.w)
            );

            vec3 N0 = rendrs_unpackOctahedron(vec2(NE0[0], NE0[1]));
            vec3 N1 = rendrs_unpackOctahedron(vec2(NE1[0], NE1[1]));
            vec3 N2 = rendrs_unpackOctahedron(vec2(NE2[0], NE2[1]));
            vec3 N = N0 * W.x + N1 * W.y + N2 * W.z;
            vec3 T = rendrs_FrameSpace_GetTanFrame(F);

            vec3  m_diffuse = pow(qDiffuseAlpha.rgb, vec3(2.2));
            float m_alpha   = qDiffuseAlpha.a;
            vec3  m_normal  = qNormalEmissive.rgb;
            float m_emit    = qNormalEmissive.a;
            float m_occlude = qOrmd.r;
            float m_rough   = qOrmd.g*qOrmd.g;
            float m_metal   = qOrmd.b;
            //float displacement = qOrmd.a;

            //todo: handedness
            vec3 B = cross(T, N);
            mat3 TBN = mat3(T, B, N);
            m_normal = m_normal * 2.0 - 1.0;
            N = normalize(TBN * m_normal);

            // material fresnel F0 evaluation
            const vec3 FRESNEL_FALLBACK = vec3(0.04);
            FresnelParams fresnel = rendrs_FresnelParams(
                m_metal, m_diffuse, FRESNEL_FALLBACK
            );
            vec3 albedo = fresnel.albedo;
            vec3 F0     = fresnel.fresnel;

            // surface outgoing radiance accumulation
            vec3 Lo = vec3(0.0);

            // ------ global view-specific ------
            vec3 to_camera = camera_position - P;
            vec3 V = normalize(to_camera);
            vec3 R = reflect(-V, N);
            float NdotV = dot(N, V);
            float absNdotV = abs(NdotV);

            // ------ local light-specific ------
            // only the one dev light is evaluated for testing

            // equal to V because this is a camera point light
            vec3 to_light = to_camera;
            vec3 L = V; // dir to light
            // test non-camera light
            // to_light = vec3(25.0, 20.0, 25.0) - P;
            // L = normalize(to_light);

            // eval. geometric angles
            float NdotL    = dot(N, L);
            float absNdotL = abs(NdotL);
            float posNdotL = max(0.0, NdotL);

            // --- light's incoming radiance ---
            const float LIGHT_MAX_DIST = 96.0;
            float light_dist_sq = dot(to_light, to_light);
            float light_dist = sqrt(light_dist_sq);
            float attenuation = rendrs_lightAttenuate(light_dist_sq, light_dist, LIGHT_MAX_DIST, 0.01);
            vec3 Li = vec3(attenuation) * 1.2;
            // light color is white

            // evaluate half-vector H, the microsurface normal
            vec3  LV     = L + V;
            float LV_len = length(LV);
            vec3  H      = LV / LV_len;

            // --- light's specular BRDF evaluation ---
            // eval. microfacet angles
            float MdotL = dot(H, L);
            float NdotM = dot(N, H);

            vec3 FS = rendrs_FresnelSchlick(max(0.0, MdotL), F0);
            vec3 kD = vec3(1.0) - FS;
            float D = rendrs_ndf_GGX(NdotM, m_rough);
            float G2 = rendrs_ndf_SmithG2_Height(absNdotV, absNdotL, m_rough);
            // G2 uses the Hammon approximation, so the denominator is no
            // longer required to be applied here on the specular term as
            // G2 already combines the denominator.
            vec3 t_FD = FS * D;
            vec3 BRDF_spec = t_FD * G2;

            vec3 BRDF_diff = albedo / 3.14159;
            vec3 BRDF = kD * BRDF_diff + BRDF_spec;
            Lo += BRDF * Li * posNdotL;
            //end light

            // --- evironmental lighting ---
            // environmental specular term
            float LODspec = m_rough * REFLECTION_MAX_LOD;
            vec3 E_spec_envf = textureLod(debug_reflection_env, R, LODspec).rgb;
            vec2 E_spec_brdf = textureLod(baked_brdf_spec, vec2(max(0.0, NdotV), m_rough), 0.0).rg;
            vec3 E_spec_FS   = F0 * E_spec_brdf.x + E_spec_brdf.y;
            vec3 E_spec = E_spec_envf * E_spec_FS;
            // environmental diffuse term
            ShCoeffs E_diff_SH = probe_irradiance;
            vec3 E_diff_L = rendrs_EvalSH_L2(E_diff_SH, N);
            vec3 E_diff = E_diff_L * albedo / 3.14159;
            vec3 E_kD   = vec3(1.0) - E_spec_FS; // diffuse energy conservation
            // environmental term evaluation
            vec3 E = E_kD * E_diff + E_spec;
            E *= m_occlude;

            vec3 color = Lo + E;

            imageStore(shade_out, id, vec4(color, 1.0));
            ";
        }
    }
}
