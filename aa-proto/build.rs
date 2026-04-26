use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto files live at the workspace root, one level above this crate.
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("aa-proto must be a direct child of the workspace root")
        .join("proto");

    // File names are relative to proto_root (the include path).
    // protox resolves imports against include paths, so passing full absolute
    // paths as file names causes "not in any include path" errors.
    let proto_files = [
        "common.proto",
        "agent.proto",
        "policy.proto",
        "audit.proto",
        "event.proto",
    ];

    // protox compiles the proto files to a FileDescriptorSet entirely in Rust —
    // no system `protoc` binary required.
    let file_descriptor_set = protox::compile(proto_files, [&proto_root])?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_fds(file_descriptor_set)?;

    // Re-run this build script if any proto file changes.
    println!("cargo:rerun-if-changed={}", proto_root.display());

    Ok(())
}
