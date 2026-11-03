//! Hands `[package.metadata.android] version-code` from Cargo.toml to the code, so the app
//! knows its own build number when it asks the PC for a newer one.

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let manifest = std::fs::read_to_string("Cargo.toml").expect("Cargo.toml is readable");
    let code = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("version-code")?.trim().strip_prefix('=')?.trim().parse::<u32>().ok())
        .expect("app/Cargo.toml needs `version-code = <n>` under [package.metadata.android]");
    println!("cargo:rustc-env=BEST_DECK_VERSION_CODE={code}");
}
