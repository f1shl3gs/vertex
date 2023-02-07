fn with_prefix(prefix: &str, content: &str) -> String {
    content
        .trim()
        .lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

pub trait GenerateConfig {
    fn generate_config() -> String;

    fn generate_config_with_indent(n: usize) -> String {
        let prefix = " ".repeat(n);

        with_prefix(&prefix, &Self::generate_config())
    }

    fn generate_commented() -> String {
        with_prefix("# ", &Self::generate_config())
    }

    fn generate_commented_with_indent(indent: usize) -> String {
        let prefix = std::iter::once('#')
            .chain(std::iter::repeat(' ').take(indent))
            .collect::<String>();

        with_prefix(&prefix, &Self::generate_config())
    }
}
