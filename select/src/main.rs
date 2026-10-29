const DEFAULT_BINPATH: &str = "bin";

#[cfg(target_os = "linux")]
const BIN_EXT: &str = "";

#[cfg(target_os = "windows")]
const BIN_EXT: &str = ".exe";

fn main() {
    let env_binpath = std::env::var("BINPATH");
    let binpath = env_binpath.unwrap_or(DEFAULT_BINPATH.to_string());
    if !std::fs::exists(&binpath).unwrap_or_default() {
        eprintln!("Binaries path {binpath} not found on filesystem.");
        return;
    }
    println!(
        "Binaries path: {}",
        std::fs::canonicalize(&binpath)
            .expect("cannot canonalize path")
            .display()
    );

    //todo: improve feature detection

    if std::arch::is_x86_feature_detected!("avx512f") {
        // x86-64-v4
        launch(&binpath, "avx512");
    } else if std::arch::is_x86_feature_detected!("avx2") {
        // x86-64-v3
        launch(&binpath, "avx2");
    } else if std::arch::is_x86_feature_detected!("sse4.2") {
        // x86-64-v2
        launch(&binpath, "sse4");
    } else if std::arch::is_x86_feature_detected!("sse2") {
        // x86-64
        launch(&binpath, "sse2");
    } else {
        eprintln!("Machine unsupported.")
    }
}

fn launch<P: AsRef<std::path::Path>>(binpath: P, feat_group: &str) {
    println!("Minimum supported feature group: {feat_group}");
    let bin_name = format!("razed-{feat_group}{BIN_EXT}");
    let fullpath = format!("{}", binpath.as_ref().join(&bin_name).display());
    println!("Launching: {fullpath}");

    let _ = std::process::Command::new(fullpath).spawn();
}
