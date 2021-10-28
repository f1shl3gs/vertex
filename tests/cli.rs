use assert_cmd::Command;

fn run_command(args: Vec<&str>) -> Vec<u8> {
    let mut cmd = Command::cargo_bin("vertex")
        .unwrap();
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output()
        .expect("Failed to execute process");

    output.stdout
}

fn assert_no_log_lines(output: Vec<u8>) {
    let output = String::from_utf8(output)
        .expect("Output is not a valid utf8 string");

    // Assert there are no lines with keywords
    let keywords = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
    for line in output.lines() {
        let present = keywords.iter()
            .any(|word| line.contains(word));
        assert!(!present, "Log detected in output line: {:?}", line);
    }
}

#[test]
fn clean_output() {
    let tests = vec![
        vec!["sources"],
        vec!["transforms"],
        vec!["sinks"],
        vec!["something_not_exist"]
    ];

    for args in tests {
        let output = run_command(args);
        assert_no_log_lines(output);
    }
}