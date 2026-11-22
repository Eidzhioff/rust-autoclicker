//! Встраивает иконку и метаданные в ресурсы `.exe` (видно в Проводнике).
//! При кросс-проверке без `windres` — мягко пропускаем, сборка не падает.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "Autoclicker (hold-to-click)");
        res.set("ProductName", "Autoclicker");
        res.set("FileVersion", "1.0.0.0");
        res.set("ProductVersion", "1.0.0.0");
        res.set("LegalCopyright", "MIT");
        if let Err(err) = res.compile() {
            println!("cargo:warning=иконка не встроена в ресурсы exe: {err}");
        }
    }
}
