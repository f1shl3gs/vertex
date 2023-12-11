use crate::compiler::parser::unescape_string;

#[derive(Debug)]
enum Segment {
    Placeholder, // represent for {}
    Literal(String),
}

#[derive(Debug)]
pub struct Template {
    segments: Vec<Segment>,
}

impl Template {
    pub fn parse(text: &str) -> Result<Template, usize> {
        let mut segments = vec![];

        let chars = text.as_bytes();
        let mut template = false;
        let mut current = String::new();
        let mut start_pos = 0;

        let mut pos = 0;
        while pos < chars.len() {
            // let ch = chars[pos] as char;
            match chars[pos] {
                b'{' if !template => {
                    start_pos = pos;
                    // start of template
                    if !current.is_empty() {
                        let seg = std::mem::take(&mut current);
                        segments.push(Segment::Literal(unescape_string(&seg)));
                    }

                    template = true;
                    pos += 1;
                }
                b'\\' if !template && (pos + 1 < chars.len() && chars[pos + 1] == b'{') => {
                    current.push('{');
                    pos += 2;
                }
                b'\\' if !template && (pos + 1 < chars.len() && chars[pos + 1] == b'}') => {
                    current.push('}');
                    pos += 2;
                }
                b'}' => {
                    // closing template
                    if template {
                        segments.push(Segment::Placeholder);
                        template = false;
                    } else {
                        // unexpected closing
                        return Err(pos);
                    }

                    pos += 1;
                }
                ch => {
                    current.push(ch as char);
                    pos += 1;
                }
            }
        }

        if template {
            Err(start_pos)
        } else {
            Ok(Template { segments })
        }
    }

    pub fn placeholders(&self) -> usize {
        self.segments.iter().fold(0, |acc, seg| match seg {
            Segment::Placeholder => acc + 1,
            Segment::Literal(_) => acc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let rs = r"";
        let input = r#"\{\} hello, " }"#;
        let result = Template::parse(input);

        println!("{:?}", result);
    }
}
