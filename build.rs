use std::collections::HashSet;
use std::env;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::Path;
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
        let result = env::var(&name).ok();
        self.envs.insert(name);
        result
    }

    pub fn emit_rerun_stanzas(&self) {
        for env in &self.envs {
            println!("cargo:rerun-if-env-changed={}", env);
        }
    }
}

struct BuildConstants {
    values: Vec<(String, String, String)>,
}

impl BuildConstants {
    pub fn new() -> Self {
        Self { values: vec![] }
    }

    pub fn add_required_constants(&mut self, name: &str, desc: &str, value: String) {
        self.values
            .push((name.to_string(), desc.to_string(), value));
    }

    pub fn write_to_file(self, file_name: impl AsRef<Path>) -> Result<()> {
        let base = std::env::var("OUT_DIR").expect("OUT_DIR not present in build script!");
        let dest = Path::new(&base).join(file_name);

        let mut output_file = std::fs::File::create(dest)?;
        output_file.write_all(
            "// AUTOGENERATED CONSTANTS. SEE BUILD>RS AT REPOSITORY ROOT. DO NOT MODIFY.\n\n"
                .as_ref(),
        )?;

        for (name, desc, value) in self.values {
            let full = format!(
                "#[doc=r#\"{}\"#]\npub const {}: &str = {:?};\n",
                desc, name, value
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

    #[cfg(feature = "sources-dnstap")]
    {
        // Generate proto if needed
        let src = std::path::PathBuf::from("src/sources/dnstap");
        let include = &[src.clone()];

        println!("cargo:rerun-if-changed=src/sources/dnstap/dnstap.proto");
        let mut config = prost_build::Config::new();
        config
            .compile_protos(&[src.join("dnstap.proto")], include)
            .unwrap();
    }

    #[cfg(feature = "sinks-loki")]
    {
        // Generate proto if needed
        let src = std::path::PathBuf::from("src/sinks/loki/proto");
        let include = &[src.clone()];

        println!("cargo:rerun-if-changed=src/sinks/loki/proto/loki.proto");
        let mut config = prost_build::Config::new();
        config
            .compile_protos(&[src.join("loki.proto")], include)
            .unwrap();
    }

    #[cfg(feature = "sinks-skywalking")]
    {
        println!("cargo:rerun-if-changed=src/sinks/skywalking/logging.proto");

        tonic_build::configure()
            .build_client(true)
            .build_server(false)
            .compile_protos(&["src/sinks/skywalking/logging.proto"], &["proto"])
            .unwrap()
    }
}

fn rustc_info() -> (String, String) {
    let rustc = env::var("RUSTC").unwrap_or_else(|_err| "rustc".to_string());

    let out = Command::new(rustc)
        .arg("-vV")
        .output()
        .expect("Get rustc version failed");

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

// Github Actions provides a lot environments for us
// https://docs.github.com/en/actions/learn-github-actions/environment-variables#default-environment-variables
fn git_info() -> Result<(String, String)> {
    let branch = match env::var("GITHUB_HEAD_REF") {
        Ok(branch) => branch,
        _ => {
            // `git branch --show-current` is easier, but it will fail when git less 2.22
            // https://github.com/actions/checkout/issues/121
            let output = Command::new("git")
                .arg("rev-parse")
                .arg("--symbolic-full-name")
                .arg("--verify")
                .arg("HEAD")
                .arg("--quit")
                .output()?;

            if !output.status.success() {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!(
                        "Unexpected exit code when get branch, stdout: {}, stderr: {}",
                        std::str::from_utf8(&output.stdout).unwrap(),
                        std::str::from_utf8(&output.stderr).unwrap(),
                    ),
                ));
            }

            let text = std::str::from_utf8(&output.stdout)
                .map_err(|err| {
                    Error::new(
                        ErrorKind::Other,
                        format!("Unexpected output when get branch, err: {}", err),
                    )
                })?
                .trim();

            match text.strip_prefix("refs/heads/") {
                Some(v) => v,
                None => match text.strip_prefix("refs/remotes/") {
                    Some(v) => v,
                    None => {
                        println!("strip branch prefix failed, branch: {}", text);
                        text
                    }
                },
            }
            .to_string()
        }
    };

    let hash = match env::var("GITHUB_SHA") {
        Ok(hash) => hash,
        _ => {
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

            std::str::from_utf8(&output.stdout)
                .map_err(|err| {
                    Error::new(
                        ErrorKind::Other,
                        format!("Unexpected output when get hash, err: {}", err),
                    )
                })?
                .trim()
                .to_string()
        }
    };

    Ok((branch, hash))
}
