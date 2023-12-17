use crate::compiler::parser::unescape_string;

#[derive(Clone, Debug)]
pub enum Segment {
    Placeholder, // represent for {}
    Literal(String),
}

#[derive(Clone, Debug)]
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
            if !current.is_empty() {
                let seg = std::mem::take(&mut current);
                segments.push(Segment::Literal(seg));
            }

            Ok(Template { segments })
        }
    }

    pub fn placeholders(&self) -> usize {
        self.segments.iter().fold(0, |acc, seg| match seg {
            Segment::Placeholder => acc + 1,
            Segment::Literal(_) => acc,
        })
    }

    #[inline]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_with_escape() {
        let input = r#"\{\} hello, " "#;
        let template = Template::parse(input).unwrap();
        assert_eq!(template.placeholders(), 0);
    }

    #[test]
    fn empty() {
        let input = r#"foo"#;
        let template = Template::parse(input).unwrap();
        assert_eq!(template.segments.len(), 1);
    }

    #[test]
    fn first() {
        let input = r#"{}bar"#;
        let template = Template::parse(input).unwrap();
        assert_eq!(template.segments.len(), 2);
        assert_eq!(template.placeholders(), 1);
    }

    #[test]
    fn middle() {
        let input = r#"foo{}bar"#;
        let template = Template::parse(input).unwrap();
        assert_eq!(template.segments.len(), 3);
        assert_eq!(template.placeholders(), 1);
    }

    #[test]
    fn end() {
        let input = r#"foo{}"#;
        let template = Template::parse(input).unwrap();
        assert_eq!(template.segments.len(), 2);
        assert_eq!(template.placeholders(), 1);
    }
}
