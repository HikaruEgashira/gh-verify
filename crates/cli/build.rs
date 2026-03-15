fn main() {
    let base = std::env::var("CARGO_PKG_VERSION").unwrap();
    let version = match std::env::var("GH_VERIFY_VERSION_SUFFIX") {
        Ok(suffix) if !suffix.is_empty() => format!("{base}-{suffix}"),
        _ => base,
    };
    println!("cargo:rustc-env=GH_VERIFY_VERSION={version}");
    println!("cargo:rerun-if-env-changed=GH_VERIFY_VERSION_SUFFIX");
}
