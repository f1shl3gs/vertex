use tail::multiline::Logic;

#[derive(Clone, Debug, Default)]
enum State {
    #[default]
    Normal,

    Stack,
}

#[derive(Clone, Debug, Default)]
pub struct Java {
    state: State,
}

impl Logic for Java {
    fn is_start(&mut self, line: &[u8]) -> bool {
        match self.state {
            State::Normal => {
                if line.starts_with(b"Exception ") {
                    self.state = State::Stack;
                    return true;
                }

                false
            }
            State::Stack => {
                if line.starts_with(b"    ") {
                    self.state = State::Stack;
                    return false;
                }

                if line.starts_with(b"Caused by") {
                    self.state = State::Stack;
                    return false;
                }

                self.state = State::Normal;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::assert_logic;
    use super::*;

    #[test]
    fn merge() {
        let input = [
            "Exception in thread \"main\" java.lang.IllegalStateException: ..null property",
            "     at com.example.myproject.Author.getBookIds(xx.java:38)",
            "     at com.example.myproject.Bootstrap.main(Bootstrap.java:14)",
            "Caused by: java.lang.NullPointerException",
            "     at com.example.myproject.Book.getId(Book.java:22)",
            "     at com.example.myproject.Author.getBookIds(Author.java:35)",
            "     ... 1 more",
            "single line",
        ];
        let expected = [
            concat! {
                "Exception in thread \"main\" java.lang.IllegalStateException: ..null property\n",
                "     at com.example.myproject.Author.getBookIds(xx.java:38)\n",
                "     at com.example.myproject.Bootstrap.main(Bootstrap.java:14)\n",
                "Caused by: java.lang.NullPointerException\n",
                "     at com.example.myproject.Book.getId(Book.java:22)\n",
                "     at com.example.myproject.Author.getBookIds(Author.java:35)\n",
                "     ... 1 more",
            },
            "single line",
        ];

        assert_logic(Java::default(), input.as_slice(), expected.as_slice());
    }
}
