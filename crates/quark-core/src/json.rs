//! Minimal JSON value + parser for the few schemas Quark actually speaks.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::Number(n) if n.is_finite() => Some(*n as i32),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            Self::String(s) => match s.to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object()?.get(key)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }

    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.get(key)?.as_i32()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)?.as_bool()
    }

    pub fn raw_display(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(b) => b.to_string(),
            Self::Number(n) => format_number(*n),
            Self::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => f.write_str(&format_number(*n)),
            Self::String(s) => {
                f.write_str("\"")?;
                f.write_str(&escape(s))?;
                f.write_str("\"")
            }
            Self::Array(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Self::Object(map) => {
                f.write_str("{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "\"{}\":{v}", escape(k))?;
                }
                f.write_str("}")
            }
        }
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn stringify_str(s: &str) -> String {
    format!("\"{}\"", escape(s))
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < (i64::MAX as f64) {
        format!("{}", n as i64)
    } else {
        let s = n.to_string();
        if s.contains('.') || s.contains('e') || s.contains('E') {
            s
        } else {
            format!("{n}.0")
        }
    }
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        i: 0,
    };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.i != p.bytes.len() {
        return Err(ParseError("trailing JSON data".into()));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.i += 1;
        Some(b)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.i += 1;
        }
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'n') => self.expect_ident(b"null").map(|_| Value::Null),
            Some(b't') => self.expect_ident(b"true").map(|_| Value::Bool(true)),
            Some(b'f') => self.expect_ident(b"false").map(|_| Value::Bool(false)),
            Some(b'"') => self.parse_string().map(Value::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b) => Err(ParseError(format!("unexpected byte {b}"))),
            None => Err(ParseError("unexpected end of JSON".into())),
        }
    }

    fn expect_ident(&mut self, ident: &[u8]) -> Result<(), ParseError> {
        if self
            .bytes
            .get(self.i..)
            .is_some_and(|s| s.starts_with(ident))
        {
            self.i += ident.len();
            Ok(())
        } else {
            Err(ParseError(format!(
                "expected {}",
                String::from_utf8_lossy(ident)
            )))
        }
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        if self.bump() != Some(b'"') {
            return Err(ParseError("expected string".into()));
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                Some(b'"') => return Ok(out),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'b') => out.push('\u{0008}'),
                    Some(b'f') => out.push('\u{000c}'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'u') => out.push(self.parse_hex_escape()?),
                    _ => return Err(ParseError("bad string escape".into())),
                },
                Some(b) if b >= 0x20 => {
                    // Restart from this byte as UTF-8.
                    self.i -= 1;
                    let rest = std::str::from_utf8(&self.bytes[self.i..])
                        .map_err(|_| ParseError("invalid utf-8 in string".into()))?;
                    let ch = rest
                        .chars()
                        .next()
                        .ok_or_else(|| ParseError("empty".into()))?;
                    out.push(ch);
                    self.i += ch.len_utf8();
                }
                Some(_) => return Err(ParseError("control character in string".into())),
                None => return Err(ParseError("unterminated string".into())),
            }
        }
    }

    fn parse_hex_escape(&mut self) -> Result<char, ParseError> {
        let mut hex = 0u32;
        for _ in 0..4 {
            let b = self
                .bump()
                .ok_or_else(|| ParseError("truncated \\u".into()))?;
            hex = (hex << 4)
                | match b {
                    b'0'..=b'9' => u32::from(b - b'0'),
                    b'a'..=b'f' => u32::from(b - b'a' + 10),
                    b'A'..=b'F' => u32::from(b - b'A' + 10),
                    _ => return Err(ParseError("bad hex in \\u".into())),
                };
        }
        char::from_u32(hex).ok_or_else(|| ParseError("invalid unicode escape".into()))
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        if self.peek() == Some(b'0') {
            self.i += 1;
        } else if matches!(self.peek(), Some(b'1'..=b'9')) {
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        } else {
            return Err(ParseError("invalid number".into()));
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(ParseError("invalid number".into()));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(ParseError("invalid number".into()));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.i])
            .map_err(|_| ParseError("invalid number".into()))?;
        let n = text
            .parse::<f64>()
            .map_err(|_| ParseError("invalid number".into()))?;
        Ok(Value::Number(n))
    }

    fn parse_array(&mut self) -> Result<Value, ParseError> {
        self.bump();
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.bump() {
                Some(b']') => return Ok(Value::Array(items)),
                Some(b',') => {
                    self.skip_ws();
                }
                _ => return Err(ParseError("expected comma or ]".into())),
            }
        }
    }

    fn parse_object(&mut self) -> Result<Value, ParseError> {
        self.bump();
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Object(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bump() != Some(b':') {
                return Err(ParseError("expected colon".into()));
            }
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.bump() {
                Some(b'}') => return Ok(Value::Object(map)),
                Some(b',') => {}
                _ => return Err(ParseError("expected comma or }".into())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_object() {
        let v = parse(r#"{"a":1,"b":"x","c":true,"d":null,"e":[1,"two"]}"#).unwrap();
        assert_eq!(v.get_i32("a"), Some(1));
        assert_eq!(v.get_str("b"), Some("x"));
        assert_eq!(v.get_bool("c"), Some(true));
        assert!(matches!(v.get("d"), Some(Value::Null)));
        assert_eq!(v.get("e").and_then(Value::as_array).unwrap().len(), 2);
        let again = parse(&v.to_string()).unwrap();
        assert_eq!(v, again);
    }
}
