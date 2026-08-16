fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    configure_windows_resources();
}

#[cfg(windows)]
fn configure_windows_resources() {
    let mut resources = winresource::WindowsResource::new();
    resources.set("ProductName", "VRCX Optimal Time App");
    resources.set("FileDescription", "VRCX Optimal Time App");
    resources.set("OriginalFilename", "VRCXOptimalTimeApp.exe");
    resources.set("ProductVersion", env!("CARGO_PKG_VERSION"));

    if let Err(error) = resources.compile() {
        panic!("failed to compile Windows resources: {error}");
    }
}
