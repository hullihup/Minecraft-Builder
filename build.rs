fn main() {
    // Embed the icon and version info into the Windows .exe.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Minecraft Builder");
        res.set(
            "FileDescription",
            "Minecraft Builder – Plugins und Mods mit Blöcken bauen",
        );
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed icon: {e}");
        }
    }
}
