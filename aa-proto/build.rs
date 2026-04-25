use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto files live at the workspace root, one level above this crate.
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("aa-proto must be a direct child of the workspace root")
        .join("proto");

    let proto_files = [
        proto_root.join("common.proto"),
        proto_root.join("agent.proto"),
        proto_root.join("policy.proto"),
        proto_root.join("audit.proto"),
        proto_root.join("event.proto"),
    ];

    // protox compiles the proto files to a FileDescriptorSet entirely in Rust —
    // no system `protoc` binary required.
    let file_descriptor_set = protox::compile(&proto_files, [&proto_root])?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_fds(file_descriptor_set)?;

    // Re-run this build script if any proto file changes.
    println!("cargo:rerun-if-changed={}", proto_root.display());

    Ok(())
}
