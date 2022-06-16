use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

struct TrackedEnv {
    envs: HashSet<String>,
}

impl TrackedEnv {
    pub fn new() -> Self {
        Self {
            envs: HashSet::new(),
        }
    }

    pub fn get_env_var(&mut self, name: impl Into<String>) -> Option<String> {
        let name = name.into();
        let result = std::env::var(&name).ok();
        self.envs.insert(name);
        result
    }

    pub fn emit_rerun_stanzas(&self) {
        for env in &self.envs {
            println!("cargo:rerun-if-env-changed={}", env);
        }
    }
}

enum ConstantValue {
    Required(String),
    Optional(Option<String>),
}

impl ConstantValue {
    pub fn as_parts(&self) -> (&'static str, String) {
        match &self {
            ConstantValue::Required(value) => ("&str", format!("{:?}", value)),
            ConstantValue::Optional(value) => match value {
                Some(value) => ("Option<&str>", format!("{:?}", value)),
                None => ("Option<&str>", "None".to_string()),
            },
        }
    }
}

struct BuildConstants {
    values: Vec<(String, String, ConstantValue)>,
}

impl BuildConstants {
    pub fn new() -> Self {
        Self { values: vec![] }
    }

    pub fn add_required_constants(&mut self, name: &str, desc: &str, value: String) {
        self.values.push((
            name.to_string(),
            desc.to_string(),
            ConstantValue::Required(value),
        ));
    }

    pub fn add_optional_constants(&mut self, name: &str, desc: &str, value: Option<String>) {
        self.values.push((
            name.to_string(),
            desc.to_string(),
            ConstantValue::Optional(value),
        ));
    }

    pub fn write_to_file(self, file_name: impl AsRef<Path>) -> std::io::Result<()> {
        let base = std::env::var("OUT_DIR").expect("OUT_DIR not present in build script!");
        let dest = Path::new(&base).join(file_name);

        let mut output_file = std::fs::File::create(dest)?;
        output_file.write_all(
            "// AUTOGENERATED CONSTANTS. SEE BUILD>RS AT REPOSITORY ROOT. DO NOT MODIFY.\n\n"
                .as_ref(),
        )?;

        for (name, desc, value) in self.values {
            let (typ, value) = value.as_parts();
            let full = format!(
                "#[doc=r#\"{}\"#]\npub const {}: {} = {};\n",
                desc, name, typ, value
            );
            output_file.write_all(full.as_ref())?;
        }

        output_file.flush()?;
        output_file.sync_all()?;

        Ok(())
    }
}

fn main() {
    // Always rerun if the build script itself changes.
    println!("cargo:rerun-if-changed=build.rs");

    // We keep track of which environment variables we slurp in, and then emit stanzas at the end
    // to inform Cargo when it needs to rerun this build script. This allows us to avoid rerunning
    // it every single time unless something ACTUALLY changes.
    let mut tracker = TrackedEnv::new();
    let pkg = tracker
        .get_env_var("CARGO_PKG_NAME")
        .expect("Cargo-provided environment variables should always exist");
    let version = tracker
        .get_env_var("CARGO_PKG_VERSION")
        .expect("Cargo-provided environment variables should always exist");
    let description = tracker
        .get_env_var("CARGO_PKG_DESCRIPTION")
        .expect("Cargo-provided environment variables should always exist");
    let target = tracker
        .get_env_var("TARGET")
        .expect("Cargo-provided environment variables should always exist");
    let arch = tracker
        .get_env_var("CARGO_CFG_TARGET_ARCH")
        .expect("Cargo-provided environment variables should always exist");
    let debug = tracker
        .get_env_var("DEBUG")
        .expect("Cargo-provided environment variables should always exist");
    let build_desc = tracker.get_env_var("VERTEX_BUILD_DESC");

    // Gather up the constants and write them out to our build constants file.
    let mut constants = BuildConstants::new();
    constants.add_required_constants("PKG_NAME", "THE full name of this package", pkg);
    constants.add_required_constants("PKG_VERSION", "The full version of this package", version);
    constants.add_required_constants(
        "PKG_DESCRIPTION",
        "The description of this package",
        description,
    );
    constants.add_required_constants(
        "TARGET",
        "The target triple being compiled for. (e.g. x86_64-unknown-linux-gnu)",
        target,
    );
    constants.add_required_constants(
        "TARGET_ARCH",
        "The target architecture being compiled for. (e.g. x86)",
        arch,
    );
    constants.add_required_constants("DEBUG", "Level of debug info for Vertex", debug);
    constants.add_optional_constants(
        "VERTEX_BUILD_DESC",
        "Special build description, related ot versioned released",
        build_desc,
    );

    let (rustc_version, rustc_channel) = rustc_info();
    constants.add_required_constants("RUSTC_VERSION", "The rustc version info", rustc_version);
    constants.add_required_constants("RUSTC_CHANNEL", "The rustc channel", rustc_channel);

    let (branch, hash) = git_info().expect("Run git command to fetch infos failed");
    constants.add_required_constants("GIT_BRANCH", "Git branch this instance built from", branch);
    constants.add_required_constants("GIT_HASH", "Git commit hash this instance built from", hash);

    constants
        .write_to_file("built.rs")
        .expect("Failed to write build-time constants file!");

    // Emit the aforementioned stanzas
    tracker.emit_rerun_stanzas();

    #[cfg(feature = "sinks-loki")]
    {
        // Generate proto if needed
        let src = PathBuf::from("src/sinks/loki/proto");
        let include = &[src.clone()];

        println!("cargo:rerun-if-changed=src/sinks/loki/proto/loki.proto");
        let mut config = prost_build::Config::new();
        config
            .compile_protos(&[src.join("loki.proto")], include)
            .unwrap();
    }
}

fn rustc_info() -> (String, String) {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));

    let out = Command::new(rustc)
        .arg("-vV")
        .output()
        .expect("Get rustc version failed");

    if !out.status.success() {}

    let output = std::str::from_utf8(&out.stdout).expect("Parse command output failed");
    let mut version = "";
    let mut channel = "";

    for line in output.lines() {
        if line.starts_with("rustc ") {
            version = line.strip_prefix("rustc ").unwrap();
            continue;
        }

        if line.starts_with("release") {
            channel = if line.contains("dev") {
                "dev"
            } else if line.contains("beta") {
                "beta"
            } else if line.contains("nightly") {
                "nightly"
            } else {
                "stable"
            }
        }
    }

    (version.to_string(), channel.to_string())
}

fn git_info() -> Result<(String, String)> {
    let output = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()?;

    if !output.status.success() {
        return Err(Error::new(
            ErrorKind::Other,
            "Unexpected exit code when get branch",
        ));
    }

    let branch = std::str::from_utf8(&output.stdout)
        .expect("Convert output to utf8 success")
        .trim()
        .to_string();

    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()?;

    if !output.status.success() {
        return Err(Error::new(
            ErrorKind::Other,
            "Unexpected exit code when get hash",
        ));
    }

    let hash = String::from_utf8(output.stdout)
        .expect("Convert output to utf8 success")
        .trim()
        .to_string();

    Ok((branch, hash))
}
