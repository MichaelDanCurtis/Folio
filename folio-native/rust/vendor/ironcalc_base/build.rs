// This vendored release has no Git checkout. Watching nonexistent .git paths
// marks the dependency dirty on every build; record its pinned identity instead.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rustc-env=GIT_VERSION={}-folio.1",
        std::env::var("CARGO_PKG_VERSION").expect("Cargo supplies the crate version")
    );
}
