fn main() {
    // The target, not cfg!(windows): a build script runs on the host.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");
        // Code signing checks ProductName against the project name and
        // ProductVersion against the release; the release workflow checks both.
        let version = env!("CARGO_PKG_VERSION");
        res.set("ProductName", "Digger")
            .set("FileDescription", "Digger")
            .set("ProductVersion", version)
            .set("FileVersion", version);
        res.compile().expect("Failed to compile Windows resources");
    }
}
