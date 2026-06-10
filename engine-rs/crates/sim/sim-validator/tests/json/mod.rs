//! A tiny std-only JSON reader for boundary fixtures.
//!
//! The engine workspace has zero external dependencies, so this test helper
//! parses the small command fixtures itself. It supports exactly the JSON the
//! generated command contracts use: objects, arrays, strings, numbers, booleans
//! and null, with the common string escapes. It is a test helper, so not every
//! accessor is exercised by every test.
#![allow(dead_code)]

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Parse a complete JSON document, erroring on trailing junk.
    pub fn parse(input: &str) -> Result<Json, String> {
        let chars: Vec<char> = input.chars().collect();
        let mut parser = Parser { chars, pos: 0 };
        parser.skip_ws();
        let value = parser.value()?;
        parser.skip_ws();
        if parser.pos != parser.chars.len() {
            return Err(format!("trailing input at position {}", parser.pos));
        }
        Ok(value)
    }

    /// Look up a key in an object value.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A non-negative integral number as `u64`.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Num(n) if n.fract() == 0.0 && *n >= 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('t' | 'f') => self.boolean(),
            Some('n') => self.null(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            other => Err(format!("unexpected {other:?} at position {}", self.pos)),
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some('}') => break,
                other => return Err(format!("expected ',' or '}}', got {other:?}")),
            }
        }
        Ok(Json::Obj(entries))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            let value = self.value()?;
            items.push(value);
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some(']') => break,
                other => return Err(format!("expected ',' or ']', got {other:?}")),
            }
        }
        Ok(Json::Arr(items))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    other => return Err(format!("unsupported escape {other:?}")),
                },
                Some(c) => out.push(c),
                None => return Err("unterminated string".to_string()),
            }
        }
        Ok(out)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while matches!(
            self.peek(),
            Some(c) if c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E' || c.is_ascii_digit()
        ) {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map(Json::Num)
            .map_err(|e| format!("bad number {text:?}: {e}"))
    }

    fn boolean(&mut self) -> Result<Json, String> {
        if self.consume("true") {
            Ok(Json::Bool(true))
        } else if self.consume("false") {
            Ok(Json::Bool(false))
        } else {
            Err(format!("invalid literal at position {}", self.pos))
        }
    }

    fn null(&mut self) -> Result<Json, String> {
        if self.consume("null") {
            Ok(Json::Null)
        } else {
            Err(format!("invalid literal at position {}", self.pos))
        }
    }

    fn consume(&mut self, literal: &str) -> bool {
        let end = self.pos + literal.len();
        if end <= self.chars.len()
            && self.chars[self.pos..end].iter().collect::<String>() == literal
        {
            self.pos = end;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        match self.bump() {
            Some(actual) if actual == c => Ok(()),
            other => Err(format!("expected {c:?}, got {other:?}")),
        }
    }
}
