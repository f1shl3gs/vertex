/*
 * MIT License
 *
 * Copyright (c) 2010 Serge Zaitsev
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

//! jsmn (pronounced like 'jasmine') is Rust port of a minimalistic JSON parser.
//! It can be easily integrated into resource-limited or embedded projects.
//!
//! # Philosophy
//!
//! Most JSON parsers offer you a bunch of functions to load JSON data, parse it and
//! extract any value by its name. jsmn proves that checking the correctness of every
//! JSON packet or allocating temporary objects to store parsed JSON fields often is
//! an overkill.
//!
//! JSON format itself is extremely simple, so why should we complicate it?
//!
//! jsmn is designed to be robust (it should work fine even with erroneous data), fast
//! (it should parse data on the fly), portable. And of course, simplicity is a key feature
//! - simple code style, simple algorithm, simple integration into other projects.

#![no_std]

use core::ops::Range;

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub size: usize,
}

impl Token {
    pub fn new(kind: TokenKind, start: Option<usize>, end: Option<usize>) -> Self {
        Self::with_size(kind, start, end, 0)
    }

    pub fn with_size(
        kind: TokenKind,
        start: Option<usize>,
        end: Option<usize>,
        size: usize,
    ) -> Self {
        Self {
            kind,
            start,
            end,
            size,
        }
    }

    pub fn as_range(&self) -> Option<Range<usize>> {
        self.start.and_then(|start| self.end.map(|end| start..end))
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    Undefined,
    Object,
    Array,
    Str,
    Primitive,
}

impl Default for TokenKind {
    fn default() -> Self {
        Self::Undefined
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Error {
    /// The string is not a full JSON packet, more bytes expected
    Part,
    /// Invalid character inside JSON string
    Invalid,
    /// Not enough tokens were provided
    NoMemory,
}

pub struct JsonParser {
    pos: usize,
    tok_next: usize,
    tok_super: Option<usize>,
}

impl Default for JsonParser {
    fn default() -> Self {
        Self {
            pos: 0,
            tok_next: 0,
            tok_super: None,
        }
    }
}

impl JsonParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.pos = 0;
        self.tok_next = 0;
        self.tok_super = None;
    }

    ///
    /// Run JSON parser. It parses a JSON data string into and array of tokens, each
    /// describing a single JSON object.
    ///
    /// Parse JSON string and fill tokens.
    ///
    /// Returns number of tokens parsed.
    pub fn parse(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<usize, Error> {
        let mut count = self.tok_next;
        while self.pos < js.len() {
            let c = js[self.pos];
            match c {
                b'{' | b'[' => {
                    count += 1;
                    let i = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    if let Some(i) = self.tok_super {
                        let t = &mut tokens[i];
                        // An object or array can't become a key
                        if let TokenKind::Object = t.kind {
                            return Err(Error::Invalid);
                        }
                        t.size += 1
                    }
                    let token = &mut tokens[i];
                    token.kind = if c == b'{' {
                        TokenKind::Object
                    } else {
                        TokenKind::Array
                    };
                    token.start = Some(self.pos);
                    self.tok_super = Some(self.tok_next - 1);
                }
                b'}' | b']' => {
                    let kind = if c == b'}' {
                        TokenKind::Object
                    } else {
                        TokenKind::Array
                    };
                    let mut i = (self.tok_next - 1) as isize;
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start.is_some() && token.end.is_none() {
                            if token.kind != kind {
                                return Err(Error::Invalid);
                            }
                            self.tok_super = None;
                            token.end = Some(self.pos + 1);
                            break;
                        } else {
                            i -= 1
                        }
                    }
                    // Error if unmatched closing bracket
                    if i == -1 {
                        return Err(Error::Invalid);
                    }
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start.is_some() && token.end.is_none() {
                            self.tok_super = Some(i as usize);
                            break;
                        } else {
                            i -= 1
                        }
                    }
                }
                b'"' => {
                    self.parse_string(js, tokens)?;
                    count += 1;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1
                    }
                }
                b'\t' | b'\r' | b'\n' | b' ' => {}
                b':' => self.tok_super = Some(self.tok_next - 1),
                b',' => {
                    if let Some(i) = self.tok_super {
                        match tokens[i].kind {
                            TokenKind::Array | TokenKind::Object => {}
                            _ => {
                                for i in (0..self.tok_next).rev() {
                                    let t = &tokens[i as usize];
                                    if let TokenKind::Array | TokenKind::Object = t.kind {
                                        if t.start.is_some() && t.end.is_none() {
                                            self.tok_super = Some(i as usize);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                b'0'..=b'9' | b'-' | b't' | b'f' | b'n' => {
                    // Primitives are: numbers and booleans and
                    // they must not be keys of the object
                    if let Some(i) = self.tok_super {
                        let t = &mut tokens[i];
                        match t.kind {
                            TokenKind::Object => return Err(Error::Invalid),
                            TokenKind::Str if t.size != 0 => return Err(Error::Invalid),
                            _ => {}
                        }
                    }
                    self.parse_primitive(js, tokens)?;
                    count += 1;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1
                    }
                }
                _ => {
                    // Unexpected char
                    return Err(Error::Invalid);
                }
            }
            self.pos += 1;
        }
        let mut i = self.tok_next as isize - 1;
        while i >= 0 {
            // Unmatched opened object or array
            if tokens[i as usize].start.is_some() && tokens[i as usize].end.is_none() {
                return Err(Error::Part);
            }
            i -= 1
        }
        Ok(count)
    }

    /// Fills next available token with JSON primitive.
    fn parse_primitive(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<(), Error> {
        let start = self.pos;
        while self.pos < js.len() {
            match js[self.pos] {
                b':' | b'\t' | b'\r' | b'\n' | b' ' | b',' | b']' | b'}' => break,
                _ => {}
            }

            if js[self.pos] < 32 || js[self.pos] >= 127 {
                self.pos = start as _;
                return Err(Error::Invalid);
            }
            self.pos += 1;
        }

        match self.alloc_token(tokens) {
            Some(i) => {
                tokens[i] = Token::new(TokenKind::Primitive, Some(start), Some(self.pos));
            }
            None => {
                self.pos = start;
                return Err(Error::NoMemory);
            }
        }

        self.pos -= 1;
        Ok(())
    }

    /// Fills next token with JSON string.
    fn parse_string(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<(), Error> {
        let start = self.pos;
        self.pos += 1;
        // Skip starting quote
        while self.pos < js.len() {
            let c = js[self.pos];
            // Quote: end of string
            if c == b'\"' {
                match self.alloc_token(tokens) {
                    Some(i) => {
                        tokens[i] = Token::new(TokenKind::Str, Some(start + 1), Some(self.pos))
                    }
                    None => {
                        self.pos = start;
                        return Err(Error::NoMemory);
                    }
                };
                return Ok(());
            }
            // Backslash: Quoted symbol expected
            if c == b'\\' && (self.pos + 1) < js.len() {
                self.pos += 1;
                match js[self.pos] {
                    b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => {}
                    b'u' => {
                        // Allows escaped symbol \uXXXX
                        self.pos += 1;
                        let mut i = 0;
                        while i < 4 && self.pos < js.len() {
                            // If it isn't a hex character we have an error

                            let is_hex = match js[self.pos] {
                                b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => true,
                                _ => false,
                            };
                            if !is_hex {
                                self.pos = start;
                                return Err(Error::Invalid);
                            }
                            self.pos += 1;
                            i += 1
                        }
                        self.pos -= 1;
                    }
                    _ => {
                        /* Unexpected symbol */
                        self.pos = start;
                        return Err(Error::Invalid);
                    }
                }
            }
            self.pos += 1;
        }
        self.pos = start as _;
        Err(Error::Part)
    }

    /// Allocates a fresh unused token from the token pool.
    fn alloc_token(&mut self, tokens: &mut [Token]) -> Option<usize> {
        if self.tok_next as usize >= tokens.len() {
            return None;
        }
        let idx = self.tok_next as usize;
        self.tok_next += 1;
        let tok = &mut tokens[idx];
        tok.end = None;
        tok.start = tok.end;
        tok.size = 0;
        Some(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! parse {
        ($buf: expr, $len: expr) => {{
            let mut v = [Token::default(); $len];
            let mut parser = JsonParser::new();
            parser.parse($buf, &mut v).map(|parsed| {
                assert_eq!($len, parsed as usize);
                v
            })
        }};
    }

    #[test]
    fn parse_int() {
        let s = b"1234";
        let tokens = parse!(s, 1).unwrap();
        assert_eq!(
            &[Token::new(TokenKind::Primitive, Some(0), Some(4))],
            &tokens
        );
    }

    #[test]
    fn parse_int_negative() {
        let s = b"-1234";
        let tokens = parse!(s, 1).unwrap();
        assert_eq!(
            &[Token::new(TokenKind::Primitive, Some(0), Some(5))],
            &tokens
        );
    }

    #[test]
    fn parse_int_invalid() {
        let s = b"abc1234";
        let err = parse!(s, 1).unwrap_err();
        assert_eq!(Error::Invalid, err);
    }

    #[test]
    fn parse_string() {
        let s = br#""abcd""#;
        let tokens = parse!(s, 1).unwrap();
        assert_eq!(&[Token::new(TokenKind::Str, Some(1), Some(5))], &tokens);
    }

    #[test]
    fn parse_object() {
        let s = br#"{"a": "b", "c": 100}"#;
        let tokens = parse!(s, 5).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Object, Some(0), Some(20), 2),
                Token::with_size(TokenKind::Str, Some(2), Some(3), 1),
                Token::with_size(TokenKind::Str, Some(7), Some(8), 0),
                Token::with_size(TokenKind::Str, Some(12), Some(13), 1),
                Token::with_size(TokenKind::Primitive, Some(16), Some(19), 0)
            ],
            &tokens
        );
    }

    #[test]
    fn parse_array() {
        let s = br#"["a", "b", "c", 100]"#;
        let tokens = parse!(s, 5).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Array, Some(0), Some(20), 4),
                Token::with_size(TokenKind::Str, Some(2), Some(3), 0),
                Token::with_size(TokenKind::Str, Some(7), Some(8), 0),
                Token::with_size(TokenKind::Str, Some(12), Some(13), 0),
                Token::with_size(TokenKind::Primitive, Some(16), Some(19), 0)
            ],
            &tokens
        );
    }

    #[test]
    fn parse_array_oom() {
        let s = br#"["a", "b", "c", 100]"#;
        let err = parse!(s, 4).unwrap_err();
        assert_eq!(Error::NoMemory, err);
    }

    #[test]
    fn parse_array_02() {
        let s = br#"["123", {"a": 1, "b": "c"}, 123]"#;
        let tokens = parse!(s, 8).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Array, Some(0), Some(32), 3),
                Token::with_size(TokenKind::Str, Some(2), Some(5), 0),
                Token::with_size(TokenKind::Object, Some(8), Some(26), 2),
                Token::with_size(TokenKind::Str, Some(10), Some(11), 1),
                Token::with_size(TokenKind::Primitive, Some(14), Some(15), 0),
                Token::with_size(TokenKind::Str, Some(18), Some(19), 1),
                Token::with_size(TokenKind::Str, Some(23), Some(24), 0),
                Token::with_size(TokenKind::Primitive, Some(28), Some(31), 0),
            ],
            &tokens
        );
    }
}
