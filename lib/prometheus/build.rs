fn main() {
    println!("cargo:rerun-if-changed=proto/prometheus-remote.proto");

    let mut prost_build = prost_build::Config::new();
    prost_build.btree_map(["."]);

    // It would be nice to just add these derives to all the types, but
    // prost automatically adds them already to enums, which causes the
    // extra derives to conflict with itself.
    prost_build.type_attribute("Label", "#[derive(Ord, PartialOrd)]");

    prost_build
        .compile_protos(&["proto/prometheus-remote.proto"], &["proto/"])
        .unwrap();
}
