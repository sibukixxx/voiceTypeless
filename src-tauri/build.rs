fn main() {
    #[cfg(target_os = "macos")]
    compile_swift();

    tauri_build::build()
}

#[cfg(target_os = "macos")]
fn compile_swift() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let swift_src = std::path::Path::new("swift-lib/Sources/VTSwift/SpeechRecognizer.swift");
    if !swift_src.exists() {
        panic!(
            "Swift source not found: {}",
            swift_src.display()
        );
    }

    // SDK パスを取得
    let sdk_output = std::process::Command::new("xcrun")
        .args(["--show-sdk-path"])
        .output()
        .expect("Failed to run xcrun --show-sdk-path");
    let sdk_path = String::from_utf8_lossy(&sdk_output.stdout)
        .trim()
        .to_string();

    // Swift コードをオブジェクトファイルにコンパイル
    let obj_path = format!("{}/SpeechRecognizer.o", out_dir);
    let status = std::process::Command::new("swiftc")
        .args([
            "-emit-object",
            "-module-name",
            "VTSwift",
            "-sdk",
            &sdk_path,
            "-parse-as-library",
            "-o",
            &obj_path,
            swift_src.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run swiftc");

    if !status.success() {
        panic!("Swift compilation failed");
    }

    // 静的ライブラリを作成
    let lib_path = format!("{}/libVTSwift.a", out_dir);
    let status = std::process::Command::new("ar")
        .args(["rcs", &lib_path, &obj_path])
        .status()
        .expect("Failed to run ar");

    if !status.success() {
        panic!("Failed to create static library");
    }

    // リンク設定
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=VTSwift");

    // Swift runtime のリンク
    let toolchain_path = "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain";
    println!(
        "cargo:rustc-link-search=native={}/usr/lib/swift/macosx",
        toolchain_path
    );
    println!("cargo:rustc-link-search=native=/usr/lib/swift");

    // macOS フレームワーク
    println!("cargo:rustc-link-lib=framework=Speech");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    println!("cargo:rustc-link-lib=framework=Foundation");

    // ソースファイル変更時の再ビルド
    println!("cargo:rerun-if-changed=swift-lib/Sources/VTSwift/SpeechRecognizer.swift");
}
