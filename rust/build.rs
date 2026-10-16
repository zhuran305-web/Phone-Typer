fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("web/icons/icon.ico");
        res.set("FileDescription", "Phone-Typer (Rust)");
        res.set("ProductName", "Phone-Typer");
        res.set("ProductVersion", "2.0.0-rust");
        if let Err(e) = res.compile() {
            println!("cargo:warning=winresource compile failed (icon not embedded): {}", e);
        }
    }
    println!("cargo:rerun-if-changed=web/icons/icon.ico");
}
