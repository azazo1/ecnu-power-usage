fn main() {
    #[cfg(target_os = "macos")]
    {
        use std::env;
        use std::path::PathBuf;
        use std::process::Command;

        println!("cargo:rerun-if-changed=src/native/notify.swift");

        // 1. 定义输出目录
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        // 2. 调用 swiftc 编译 Swift 文件为静态库
        // -static-stdlib: 包含 Swift 运行时
        // -emit-library: 生成库文件
        let status = Command::new("swiftc")
            .args([
                "-emit-library",
                "-static",
                "src/native/notify.swift", // Swift 文件路径
                "-o",
                out_dir.join("libnotify.a").to_str().unwrap(),
            ])
            .status()
            .expect("Failed to compile Swift code");

        if !status.success() {
            panic!("Swift compiler failed");
        }

        // 3. 告诉 Rust 去哪里找这个库
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        // 4. 链接这个库 (libnotify.a -> notify)
        println!("cargo:rustc-link-lib=static=notify");

        // 5. 必须链接 macOS 的系统框架
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=UserNotifications");
        println!("cargo:rustc-link-lib=framework=AppKit");
        // 链接 Swift 运行时
        println!("cargo:rustc-link-lib=swiftCore");
        println!("cargo:rustc-link-lib=swiftFoundation");
    }

    tauri_build::build()
}
