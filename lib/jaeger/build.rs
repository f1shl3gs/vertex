fn main() {
    println!("cargo:rerun-if-changed=proto/collector.proto");
    println!("cargo:rerun-if-changed=proto/model.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/collector.proto", "proto/model.proto"], &["proto"])
        .unwrap()
}
