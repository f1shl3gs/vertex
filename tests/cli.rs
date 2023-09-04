use assert_cmd::Command;
use std::process::ExitStatus;

fn run_command(args: Vec<&str>) -> (Vec<u8>, ExitStatus) {
    let mut cmd = Command::cargo_bin("vertex").unwrap();
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().expect("Failed to execute process");

    (output.stdout, output.status)
}

fn assert_no_log_lines(output: Vec<u8>) {
    let output = String::from_utf8(output).expect("Output is not a valid utf8 string");

    // Assert there are no lines with keywords
    let keywords = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
    for line in output.lines() {
        let present = keywords.iter().any(|word| line.contains(word));
        assert!(!present, "Log detected in output line: {:?}", line);
    }
}

#[test]
fn clean_output() {
    let tests = vec![
        (vec!["sources"], true),
        (vec!["sources", "node"], true),
        (vec!["sources", "something_not_exist"], false),
        (vec!["transforms"], true),
        (vec!["transforms", "rewrite"], true),
        (vec!["transforms", "something_not_exist"], false),
        (vec!["sinks"], true),
        (vec!["sinks", "something_not_exist"], false),
        (vec!["extensions"], true),
        (vec!["extensions", "something_not_exist"], false),
        (vec!["something_not_exist"], false),
    ];

    for (args, want) in tests {
        let (output, status) = run_command(args.clone());
        assert_no_log_lines(output);
        assert_eq!(status.success(), want, "args: {:?}", args)
    }
}

/// Validate example configs will help us keep them updated.
#[test]
fn validate_example_configs() {
    let mut dir = std::fs::read_dir("examples/config").unwrap();

    // Clippy tell us use `for in`, if we do use `for in`, then
    // it tell us not to use `for in`.
    #[allow(clippy::while_let_on_iterator)]
    while let Some(result) = dir.next() {
        let entry = result.expect("Scan entry failed");

        let path = entry.path().to_string_lossy().to_string();
        if !path.ends_with(".yaml") && !path.ends_with(".yml") {
            continue;
        }

        let args = vec!["validate", "-c", &path, "--no-environment"];
        let (output, status) = run_command(args.clone());
        assert_no_log_lines(output.clone());
        assert!(
            status.success(),
            "args: {:?}\noutput: {:?}",
            args,
            String::from_utf8_lossy(&output)
        );
    }
}
