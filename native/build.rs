fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        println!("cargo:rerun-if-changed=icon.rc");
        println!("cargo:rerun-if-changed=icon.ico");
        embed_resource::compile("icon.rc", None::<&str>);
    }
}
