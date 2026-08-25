fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
