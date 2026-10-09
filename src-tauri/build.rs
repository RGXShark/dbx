mod enterprise_ai_build;
#[path = "src/enterprise_ai_schema.rs"]
mod enterprise_ai_schema;

fn main() {
    println!("cargo:rerun-if-env-changed=DBX_ENTERPRISE_AI_CONFIG");
    println!("cargo:rerun-if-changed=src/enterprise_ai_schema.rs");
    println!("cargo:rerun-if-changed=enterprise_ai_build.rs");
    let path = std::env::var_os("DBX_ENTERPRISE_AI_CONFIG").map(std::path::PathBuf::from);
    if let Some(path) = &path {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    enterprise_ai_build::embed(path.as_deref(), &out).expect("Invalid enterprise AI build configuration");
    let is_win7_target = std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() == Ok("win7");
    if is_win7_target && std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_none() {
        panic!("Windows 7 release builds must enable the custom-protocol feature");
    }
    tauri_build::build()
}
