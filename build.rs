fn main() {
    // Bake the build date into the binary for 30-day trial enforcement.
    // No key required at install; trial window is 30 days from this date.
    let output = std::process::Command::new("sh")
        .args(["-c", "date +%Y-%m-%d"])
        .output()
        .expect("failed to get build date");
    let date = String::from_utf8(output.stdout).expect("date output is not UTF-8");
    println!("cargo:rustc-env=VEYRON_BUILD_DATE={}", date.trim());
    println!("cargo:rerun-if-changed=build.rs");
}
