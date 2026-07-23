fn main() {
    tauri_build::build();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=native/voice_recognizer.m");
        cc::Build::new()
            .file("native/voice_recognizer.m")
            .flag("-fobjc-arc")
            .compile("ph7_voice_recognizer");

        for framework in ["Foundation", "AVFoundation", "Speech"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}
