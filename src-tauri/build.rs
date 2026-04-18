fn main() {
    tauri_build::build();

    // 启用 media feature 时，静态链接 libav（FFmpeg）依赖的 Windows 系统库。
    // ffmpeg-sys-next 8.1 的 build.rs 在 FFMPEG_DIR 模式下**不会**自动补这些，
    // 需要我们显式声明，否则链接时报：
    //   avcodec.lib(mfenc.o) : unresolved external symbol IID_ICodecAPI
    //   ... IID_IMFMediaEventGenerator / IID_IMFTransform 等
    // 这些是 Windows Media Foundation 的 COM interface IDs，在 mfuuid.lib 里。
    #[cfg(all(target_os = "windows", feature = "media"))]
    {
        println!("cargo:rustc-link-lib=mfuuid");
        println!("cargo:rustc-link-lib=strmiids");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=secur32");
        println!("cargo:rustc-link-lib=ws2_32");
        // openh264 + libmfx(QSV) 静态库需要显式链接
        println!("cargo:rustc-link-lib=static=openh264");
        println!("cargo:rustc-link-lib=static=libmfx");
        if let Ok(dir) = std::env::var("FFMPEG_DIR") {
            println!("cargo:rustc-link-search=native={}/lib", dir);
        }
    }
}
