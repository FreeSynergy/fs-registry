fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto = "proto/registry.proto";
    println!("cargo:rerun-if-changed={proto}");

    let protoc = protoc_bin_vendored::protoc_bin_path().map_err(|e| format!("protoc: {e}"))?;
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos_with_config(prost_build::Config::new(), &[proto], &["proto/"])?;

    Ok(())
}
