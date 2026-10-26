#[test]
fn generated_paths_resolve_a_renamed_published_meta_dependency() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::process::Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--offline", "--manifest-path"])
        .arg(root.join("tests/fixtures/renamed/Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target/renamed-fixture"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "renamed dependency fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
