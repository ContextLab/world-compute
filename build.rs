fn main() -> Result<(), Box<dyn std::error::Error>> {
    // gRPC proto compilation
    tonic_build::configure().build_server(true).build_client(true).compile_protos(
        &[
            "proto/donor.proto",
            "proto/submitter.proto",
            "proto/cluster.proto",
            "proto/governance.proto",
            "proto/admin.proto",
            "proto/mesh_llm.proto",
        ],
        &["proto/"],
    )?;

    // FR-S051: Embed provenance metadata into the binary for attestation.
    // This allows the binary to self-report its build origin for verification.
    println!(
        "cargo:rustc-env=WC_BUILD_TIMESTAMP={}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    if let Ok(hash) = std::env::var("GIT_COMMIT_HASH") {
        println!("cargo:rustc-env=WC_GIT_COMMIT={hash}");
    } else if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
    {
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !hash.is_empty() {
            println!("cargo:rustc-env=WC_GIT_COMMIT={hash}");
        }
    }
    println!(
        "cargo:rustc-env=WC_RUSTC_VERSION={}",
        std::env::var("RUSTC_WRAPPER")
            .unwrap_or_else(|_| "rustc".to_string())
    );

    Ok(())
}
