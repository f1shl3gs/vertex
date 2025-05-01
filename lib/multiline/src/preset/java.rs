use bytes::Bytes;

use crate::aggregate::{Mode, Rule};

#[derive(Debug, Default)]
enum State {
    #[default]
    Normal,
    Excepting,
}

#[derive(Default)]
pub struct Java {
    state: State,
}

impl Rule for Java {
    fn is_start(&mut self, line: &Bytes) -> bool {
        match self.state {
            State::Normal => {
                if line.starts_with(b"Exception in thread ") {
                    self.state = State::Excepting;
                    return true;
                }

                false
            }
            State::Excepting => false,
        }
    }

    fn is_condition(&mut self, line: &Bytes) -> bool {
        match self.state {
            State::Normal => {
                if line.starts_with(b"Exception in thread") {
                    self.state = State::Excepting;
                }

                true
            }
            State::Excepting => {
                if line[0].is_ascii_whitespace() {
                    return true;
                }

                if !line.starts_with(b"Caused by:") {
                    self.state = State::Normal;
                    return false;
                }

                true
            }
        }
    }

    fn mode(&self) -> Mode {
        Mode::ContinueThrough
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::assert_rule;

    #[tokio::test]
    async fn merge() {
        let input = [
            "first line",
            "Exception in thread \"main\" java.lang.IllegalStateException: ..null property",
            "     at com.example.myproject.Author.getBookIds(xx.java:38)",
            "     at com.example.myproject.Bootstrap.main(Bootstrap.java:14)",
            "Caused by: java.lang.NullPointerException",
            "     at com.example.myproject.Book.getId(Book.java:22)",
            "     at com.example.myproject.Author.getBookIds(Author.java:35)",
            "     ... 1 more",
            "single line",
        ];

        let output = [
            "first line",
            r#"Exception in thread "main" java.lang.IllegalStateException: ..null property
     at com.example.myproject.Author.getBookIds(xx.java:38)
     at com.example.myproject.Bootstrap.main(Bootstrap.java:14)
Caused by: java.lang.NullPointerException
     at com.example.myproject.Book.getId(Book.java:22)
     at com.example.myproject.Author.getBookIds(Author.java:35)
     ... 1 more"#,
            "single line",
        ];

        assert_rule(
            &input,
            &output,
            Java {
                state: State::Normal,
            },
        )
        .await;
    }
}
