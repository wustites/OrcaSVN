fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            "x86" => "ia32",
            other => panic!("Unsupported Windows SVN architecture: {other}"),
        };
        let archive = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join(format!("runtime/windows-{arch}.zip"));
        assert!(
            archive.is_file(),
            "Run npm run prepare:svn before compiling the Windows app"
        );
        println!("cargo:rerun-if-changed={}", archive.display());
        println!(
            "cargo:rustc-env=ORCASVN_RUNTIME_ARCHIVE={}",
            archive.display()
        );
    }
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/32x32.png");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
