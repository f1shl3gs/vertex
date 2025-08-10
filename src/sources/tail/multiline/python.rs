use tail::multiline::Logic;

#[derive(Clone, Debug, Default)]
enum State {
    #[default]
    Normal,

    File,
    Code,
}

#[derive(Clone, Debug, Default)]
pub struct Python {
    state: State,
}

impl Logic for Python {
    fn is_start(&mut self, line: &[u8]) -> bool {
        match self.state {
            State::Normal => {
                if line.starts_with(b"Traceback ") {
                    self.state = State::File;
                }

                true
            }
            State::File => {
                if line.starts_with(b"  File") {
                    self.state = State::Code;
                    return false;
                }

                self.state = State::Normal;

                !line.starts_with(b"Exception: ")
            }
            State::Code => {
                if line.starts_with(b"    ") {
                    self.state = State::File;
                    false
                } else {
                    true
                }
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
            "Traceback (most recent call last):",
            "  File \"/base/data/home/runtimes/python27/python27_lib/versions/third_party/webapp2-2.5.2/webapp2.py\", line 1535, in __call__",
            "    rv = self.handle_exception(request, response, e)",
            "  File \"/base/data/home/apps/s~nearfieldspy/1.378705245900539993/nearfieldspy.py\", line 17, in start",
            "    return get()",
            "  File \"/base/data/home/apps/s~nearfieldspy/1.378705245900539993/nearfieldspy.py\", line 5, in get",
            "    raise Exception('spam', 'eggs')",
            "Exception: ('spam', 'eggs')",
            "hello world, not multiline",
        ];
        let expected = [
            concat!(
                "Traceback (most recent call last):\n",
                "  File \"/base/data/home/runtimes/python27/python27_lib/versions/third_party/webapp2-2.5.2/webapp2.py\", line 1535, in __call__\n",
                "    rv = self.handle_exception(request, response, e)\n",
                "  File \"/base/data/home/apps/s~nearfieldspy/1.378705245900539993/nearfieldspy.py\", line 17, in start\n",
                "    return get()\n",
                "  File \"/base/data/home/apps/s~nearfieldspy/1.378705245900539993/nearfieldspy.py\", line 5, in get\n",
                "    raise Exception('spam', 'eggs')\n",
                "Exception: ('spam', 'eggs')",
            ),
            "hello world, not multiline",
        ];

        assert_logic(Python::default(), input.as_slice(), expected.as_slice());
    }
}
