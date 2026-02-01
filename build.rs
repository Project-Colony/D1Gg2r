fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("src/ui/assets/icons/digger.ico");
        res.compile().expect("Failed to compile Windows resources");
    }
}
