macro_rules! precompile_shader_compute {
    ($st:ty, $var:expr) => {
        {
            let name = <$st>::HASHED_NAME;
            let hashed = <$st>::hash_sources();

            let filename = format!("{name:#x}-{}.comp", $var);
            let glsl_source_path = format!("{SHADERS_SOURCE_DIR}/{filename}");
            let spirv_source_path = format!("{SHADERS_SOURCE_SPIRV_DIR}/{filename}");

            if let Ok(cache_hashed_str) =
                std::fs::read_to_string(format!("{SHADERS_SOURCE_DIR}/hash.{filename}"))
                && let Ok(cache_hashed) = cache_hashed_str.parse::<u64>()
                // glsl source has not changed
                && cache_hashed == hashed
                // but check if spirv exists
                && std::fs::exists(&spirv_source_path).unwrap_or_default()
            {
                println!("skipping redundant work for {} (variant={})", stringify!($st), $var);
            } else {
                // write glsl and recompile spirv
                println!("compiling compute {glsl_source_path}");
                <$st>::save_sources();
                match std::process::Command::new(GLSLANG_PATH)
                    .arg("-G")
                    .arg("-Os")
                    .arg("-g0")
                    .arg("--lto")
                    .arg(&glsl_source_path)
                    .arg("-o")
                    .arg(&spirv_source_path)
                    .spawn()
                {
                    Ok(mut p) => {
                        p.wait().unwrap();
                    }
                    Err(err) => eprintln!("failed to compile compute spir-v source from glsl: {err}"),
                }
                println!("spirv compute compiled at {spirv_source_path}");
            }
        }
    };
}

const SHADERS_SOURCE_DIR: &str = "target/shaders/glsl";
const SHADERS_SOURCE_SPIRV_DIR: &str = "target/shaders/spirv";

#[cfg(target_os = "linux")]
const GLSLANG_PATH: &str = "thirdparty/glslang/linux/glslang";
#[cfg(target_os = "windows")]
const GLSLANG_PATH: &str = "thirdparty/glslang/windows/glslang.exe";

fn main() {
    let _ = std::fs::create_dir_all(SHADERS_SOURCE_SPIRV_DIR);

    precompile_shader_compute!(
        razed::render::geometry::ComputeShaderFragmentsGeomSubmit,
        razed::render::geometry::ComputeShaderFragmentsGeomSubmitVariants::Default
    );
    precompile_shader_compute!(
        razed::render::geometry::ComputeShaderDebrisGeomSubmit,
        razed::render::geometry::ComputeShaderDebrisGeomSubmitVariants::Default
    );
    precompile_shader_compute!(
        razed::render::geometry::ComputeShaderPlaneGeomSubmit,
        razed::render::geometry::ComputeShaderPlaneGeomSubmitVariants::Default
    );
    precompile_shader_compute!(
        razed::render::pass::ComputeShaderTonemap,
        razed::render::pass::ComputeShaderTonemapVariants::Default
    );
}
