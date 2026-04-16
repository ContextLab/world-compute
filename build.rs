fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}
