use std::process::{Command, ExitStatus};

fn run_command(args: Vec<&str>) -> (Vec<u8>, ExitStatus) {
    let output = Command::new(env!("CARGO_BIN_EXE_vertex"))
        .args(args)
        .output()
        .expect("Failed to execute process");

    (output.stdout, output.status)
}

fn assert_no_log_lines(output: &[u8]) {
    let output = String::from_utf8_lossy(output);

    // Assert there are no lines with keywords
    let keywords = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
    for line in output.lines() {
        let present = keywords.iter().any(|word| line.contains(word));
        assert!(!present, "Log detected in output line: {line:?}");
    }
}

#[test]
fn clean_output() {
    let tests = vec![
        (vec!["sources"], true),
        (vec!["sources", "node"], true),
        (vec!["sources", "something_not_exist"], false),
        (vec!["transforms"], true),
        (vec!["transforms", "relabel"], true),
        (vec!["transforms", "something_not_exist"], false),
        (vec!["sinks"], true),
        (vec!["sinks", "something_not_exist"], false),
        (vec!["extensions"], true),
        (vec!["extensions", "something_not_exist"], false),
        (vec!["something_not_exist"], false),
    ];

    for (args, want) in tests {
        let (output, status) = run_command(args.clone());
        assert_no_log_lines(&output);
        assert_eq!(status.success(), want, "args: {args:?}")
    }
}

/// Validate example configs will help us keep them updated.
#[test]
fn validate_example_configs() {
    let dir = std::fs::read_dir("examples").unwrap();

    for entry in dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // TODO: fix this later
        if path.file_name().unwrap().to_string_lossy() == "http_provider.yml" {
            continue;
        }

        if let Some(ext) = path.extension() {
            if ext != "yaml" && ext != "yml" {
                continue;
            }

            let args = vec!["validate", "-c", path.to_str().unwrap(), "--no-environment"];
            let (output, status) = run_command(args.clone());
            assert_no_log_lines(&output);
            assert!(
                status.success(),
                "args: {:?}\noutput: {:?}",
                args,
                String::from_utf8_lossy(&output)
            );
        }
    }
}
