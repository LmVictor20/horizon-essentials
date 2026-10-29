// horizon-essentials: the YAML that EssentialsX's files use (config.yml, kits.yml, userdata, plugin.yml),
// read into and written from serde_json::Value. A subset of YAML 1.1 as SnakeYAML reads these files:
// block maps and sequences by indentation, flow `[a, b]` and `{k: v}`, plain/'single'/"double" scalars,
// `|` and `>` block scalars, `#` comments, documents of one value. Anchors and tags are not supported.

use serde_json::{Map, Number, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct YamlError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for YamlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

struct Line {
    number: usize,
    indent: usize,
    text: String,
}

/// Strips a `#` comment that starts outside quotes (and after whitespace, as YAML requires).
fn strip_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let (mut single, mut double) = (false, false);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' if !double => single = !single,
            b'"' if !single => double = !double,
            b'\\' if double => i += 1,
            b'#' if !single && !double && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') => {
                return s[..i].trim_end();
            }
            _ => {}
        }
        i += 1;
    }
    s.trim_end()
}

pub fn parse(source: &str) -> Result<Value, YamlError> {
    let raw: Vec<String> = source.lines().map(str::to_string).collect();
    let mut p = Parser { raw, pos: 0 };
    p.skip_blank();
    if p.pos >= p.raw.len() {
        return Ok(Value::Null);
    }
    let first = p.line(p.pos);
    if first.text == "---" {
        p.pos += 1;
        p.skip_blank();
        if p.pos >= p.raw.len() {
            return Ok(Value::Null);
        }
    }
    let indent = p.line(p.pos).indent;
    p.block(indent)
}

struct Parser {
    raw: Vec<String>,
    pos: usize,
}

impl Parser {
    fn line(&self, i: usize) -> Line {
        let full = self.raw[i].as_str();
        let indent = full.len() - full.trim_start_matches(' ').len();
        Line { number: i + 1, indent, text: strip_comment(full[indent..].trim_end_matches('\r')).to_string() }
    }

    fn skip_blank(&mut self) {
        while self.pos < self.raw.len() {
            let l = self.line(self.pos);
            if !l.text.is_empty() && l.text != "..." {
                break;
            }
            self.pos += 1;
        }
    }

    fn err<T>(&self, line: usize, message: impl Into<String>) -> Result<T, YamlError> {
        Err(YamlError { line, message: message.into() })
    }

    /// A block node whose lines are indented exactly `indent`.
    fn block(&mut self, indent: usize) -> Result<Value, YamlError> {
        self.skip_blank();
        if self.pos >= self.raw.len() {
            return Ok(Value::Null);
        }
        let l = self.line(self.pos);
        if l.text == "-" || l.text.starts_with("- ") {
            self.sequence(indent)
        } else if split_key(&l.text).is_some() {
            self.mapping(indent)
        } else {
            // A lone scalar (possibly a flow collection over several lines).
            self.pos += 1;
            let mut text = l.text.to_string();
            while open_flow(&text) && self.pos < self.raw.len() {
                text.push(' ');
                text.push_str(&self.line(self.pos).text);
                self.pos += 1;
            }
            scalar_or_flow(&text, l.number)
        }
    }

    fn mapping(&mut self, indent: usize) -> Result<Value, YamlError> {
        let mut map = Map::new();
        loop {
            self.skip_blank();
            if self.pos >= self.raw.len() {
                break;
            }
            let l = self.line(self.pos);
            if l.indent < indent {
                break;
            }
            if l.indent > indent {
                return self.err(l.number, "unexpected indentation");
            }
            if l.text.starts_with("- ") || l.text == "-" {
                break;
            }
            let Some((key, rest)) = split_key(&l.text) else {
                return self.err(l.number, format!("expected `key: value`, found {:?}", l.text));
            };
            self.pos += 1;
            let value = self.value_after(rest, indent, l.number, true)?;
            map.insert(key, value);
        }
        Ok(Value::Object(map))
    }

    fn sequence(&mut self, indent: usize) -> Result<Value, YamlError> {
        let mut items = Vec::new();
        loop {
            self.skip_blank();
            if self.pos >= self.raw.len() {
                break;
            }
            let l = self.line(self.pos);
            if l.indent != indent || !(l.text == "-" || l.text.starts_with("- ")) {
                if l.indent > indent {
                    return self.err(l.number, "unexpected indentation");
                }
                break;
            }
            let rest = l.text[1..].trim_start();
            if rest.is_empty() {
                self.pos += 1;
                items.push(self.nested(indent, l.number)?);
            } else if split_key(rest).is_some() && !rest.starts_with(['"', '\'', '[', '{']) {
                // `- key: value` starts a map whose keys sit at the column after "- ".
                let column = indent + (l.text.len() - rest.len());
                self.raw_rewrite_first(column, rest);
                items.push(self.mapping(column)?);
            } else {
                self.pos += 1;
                items.push(self.value_after(rest, indent, l.number, false)?);
            }
        }
        Ok(Value::Array(items))
    }

    /// The first key of a `- key: value` item is on the dash line: parse it as if it began its own line.
    fn raw_rewrite_first(&mut self, column: usize, rest: &str) {
        self.raw[self.pos] = format!("{}{}", " ".repeat(column), rest);
    }

    /// What follows `key:` or `- `: an inline scalar/flow value, a block scalar, or a nested block.
    fn value_after(&mut self, rest: &str, indent: usize, number: usize, in_map: bool) -> Result<Value, YamlError> {
        if rest.is_empty() {
            return self.nested_or_null(indent, number, in_map);
        }
        if let Some(style) = rest.strip_prefix('|').map(|s| ('|', s)).or_else(|| rest.strip_prefix('>').map(|s| ('>', s))) {
            if style.1.chars().all(|c| matches!(c, '-' | '+' | '0'..='9')) {
                return Ok(Value::String(self.block_scalar(indent, style.0, style.1)));
            }
        }
        let mut text = rest.to_string();
        while open_flow(&text) && self.pos < self.raw.len() {
            text.push(' ');
            text.push_str(&self.line(self.pos).text);
            self.pos += 1;
        }
        // A plain scalar may continue on more-indented lines.
        if !text.starts_with(['"', '\'', '[', '{']) {
            while self.pos < self.raw.len() {
                let next = self.line(self.pos);
                if next.text.is_empty() || next.indent <= indent || split_key(&next.text).is_some() || next.text.starts_with("- ") {
                    break;
                }
                text.push(' ');
                text.push_str(&next.text);
                self.pos += 1;
            }
        } else if text.starts_with('"') && !closed_double(&text) {
            while self.pos < self.raw.len() && !closed_double(&text) {
                text.push(' ');
                text.push_str(&self.line(self.pos).text);
                self.pos += 1;
            }
        }
        scalar_or_flow(&text, number)
    }

    fn nested_or_null(&mut self, indent: usize, number: usize, in_map: bool) -> Result<Value, YamlError> {
        self.skip_blank();
        if self.pos >= self.raw.len() {
            return Ok(Value::Null);
        }
        let next = self.line(self.pos);
        // A map's value may be a sequence at the same indentation as the key (`key:\n- a`).
        if next.indent > indent || (in_map && next.indent == indent && (next.text.starts_with("- ") || next.text == "-")) {
            return self.block(next.indent);
        }
        let _ = number;
        Ok(Value::Null)
    }

    fn nested(&mut self, indent: usize, number: usize) -> Result<Value, YamlError> {
        self.nested_or_null(indent, number, false)
    }

    fn block_scalar(&mut self, indent: usize, style: char, header: &str) -> String {
        let keep = header.contains('+');
        let strip = header.contains('-');
        let mut lines: Vec<String> = Vec::new();
        let mut block_indent = None;
        while self.pos < self.raw.len() {
            let full = self.raw[self.pos].clone();
            let trimmed = full.trim_start_matches(' ');
            let this_indent = full.len() - trimmed.len();
            if trimmed.trim().is_empty() {
                lines.push(String::new());
                self.pos += 1;
                continue;
            }
            if this_indent <= indent {
                break;
            }
            let bi = *block_indent.get_or_insert(this_indent);
            if this_indent < bi {
                break;
            }
            lines.push(full[bi..].trim_end_matches('\r').to_string());
            self.pos += 1;
        }
        let trailing = lines.iter().rev().take_while(|l| l.is_empty()).count();
        let body = &lines[..lines.len() - trailing];
        let mut out = if style == '|' {
            body.join("\n")
        } else {
            let mut s = String::new();
            for (i, l) in body.iter().enumerate() {
                if i > 0 {
                    s.push(if l.is_empty() || body[i - 1].is_empty() { '\n' } else { ' ' });
                }
                s.push_str(l);
            }
            s
        };
        if !strip && !body.is_empty() {
            out.push('\n');
            if keep {
                out.push_str(&"\n".repeat(trailing));
            }
        }
        out
    }
}

fn closed_double(s: &str) -> bool {
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if i == 0 {
            continue;
        }
        match c {
            '\\' if !escaped => escaped = true,
            '"' if !escaped => return true,
            _ => escaped = false,
        }
    }
    false
}

/// `key: rest` (or `key:` alone): the key unquoted and the rest trimmed.
fn split_key(text: &str) -> Option<(String, &str)> {
    if text.starts_with('[') || text.starts_with('{') {
        return None;
    }
    let (key, after) = if let Some(q) = text.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let end = quoted_end(text, q)?;
        let after = &text[end..];
        let after = after.trim_start();
        let after = after.strip_prefix(':')?;
        (unquote(&text[..end]), after)
    } else {
        let mut idx = None;
        for (i, c) in text.char_indices() {
            if c == ':' && (i + 1 == text.len() || text[i + 1..].starts_with([' ', '\t'])) {
                idx = Some(i);
                break;
            }
        }
        let i = idx?;
        (text[..i].trim_end().to_string(), &text[i + 1..])
    };
    if !after.is_empty() && !after.starts_with([' ', '\t']) {
        return None;
    }
    Some((key, after.trim()))
}

fn quoted_end(text: &str, q: char) -> Option<usize> {
    let mut chars = text.char_indices().skip(1).peekable();
    while let Some((i, c)) = chars.next() {
        if q == '"' && c == '\\' {
            chars.next();
        } else if c == q {
            if q == '\'' && chars.peek().is_some_and(|(_, n)| *n == '\'') {
                chars.next();
            } else {
                return Some(i + 1);
            }
        }
    }
    None
}

fn open_flow(text: &str) -> bool {
    let t = text.trim_start();
    if !(t.starts_with('[') || t.starts_with('{')) {
        return false;
    }
    let mut depth = 0i32;
    let (mut single, mut double, mut escaped) = (false, false, false);
    for c in t.chars() {
        if double {
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => double = false,
                _ => escaped = false,
            }
            continue;
        }
        match c {
            '\'' => single = !single,
            '"' if !single => double = true,
            '[' | '{' if !single => depth += 1,
            ']' | '}' if !single => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

fn scalar_or_flow(text: &str, line: usize) -> Result<Value, YamlError> {
    let t = text.trim();
    if t.starts_with('[') || t.starts_with('{') {
        let mut f = Flow { s: t.as_bytes(), src: t, i: 0, line };
        let v = f.value()?;
        f.ws();
        if f.i != f.s.len() {
            return Err(YamlError { line, message: format!("trailing text after flow value: {:?}", &t[f.i..]) });
        }
        return Ok(v);
    }
    Ok(scalar(t))
}

struct Flow<'s> {
    s: &'s [u8],
    src: &'s str,
    i: usize,
    line: usize,
}

impl Flow<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }
    fn fail<T>(&self, m: &str) -> Result<T, YamlError> {
        Err(YamlError { line: self.line, message: format!("{m} in flow value {:?}", self.src) })
    }
    fn value(&mut self) -> Result<Value, YamlError> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    self.ws();
                    if self.s.get(self.i) == Some(&b']') {
                        self.i += 1;
                        return Ok(Value::Array(items));
                    }
                    items.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {}
                        _ => return self.fail("expected , or ]"),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut map = Map::new();
                loop {
                    self.ws();
                    if self.s.get(self.i) == Some(&b'}') {
                        self.i += 1;
                        return Ok(Value::Object(map));
                    }
                    let key = match self.value()? {
                        Value::String(s) => s,
                        other => other.to_string(),
                    };
                    self.ws();
                    let value = if self.s.get(self.i) == Some(&b':') {
                        self.i += 1;
                        self.value()?
                    } else {
                        Value::Null
                    };
                    map.insert(key, value);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {}
                        _ => return self.fail("expected , or }"),
                    }
                }
            }
            Some(q @ (b'"' | b'\'')) => {
                let q = *q as char;
                let rest = &self.src[self.i..];
                let Some(end) = quoted_end(rest, q) else { return self.fail("unterminated string") };
                self.i += end;
                Ok(Value::String(unquote(&rest[..end])))
            }
            Some(_) => {
                let start = self.i;
                while self.i < self.s.len() {
                    let c = self.s[self.i];
                    if c == b',' || c == b']' || c == b'}' {
                        break;
                    }
                    if c == b':' && self.s.get(self.i + 1).is_none_or(|n| (*n as char).is_whitespace()) {
                        break;
                    }
                    self.i += 1;
                }
                Ok(scalar(self.src[start..self.i].trim()))
            }
            None => self.fail("unexpected end"),
        }
    }
}

fn unquote(q: &str) -> String {
    if q.len() >= 2 && q.starts_with('\'') && q.ends_with('\'') {
        return q[1..q.len() - 1].replace("''", "'");
    }
    if q.len() >= 2 && q.starts_with('"') && q.ends_with('"') {
        let inner = &q[1..q.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('0') => out.push('\0'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some(' ') => out.push(' '),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(ch);
                    }
                }
                Some('x') => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(ch);
                    }
                }
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        }
        return out;
    }
    q.to_string()
}

/// A plain or quoted scalar, typed as SnakeYAML types it (YAML 1.1 core types).
fn scalar(t: &str) -> Value {
    if t.starts_with('"') || t.starts_with('\'') {
        return Value::String(unquote(t));
    }
    match t {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" => return Value::Bool(true),
        "false" | "False" | "FALSE" | "no" | "No" | "NO" | "off" | "Off" | "OFF" => return Value::Bool(false),
        _ => {}
    }
    let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit() || b == b'_') && !digits.starts_with('_') {
        if let Ok(n) = t.replace('_', "").parse::<i64>() {
            return Value::Number(n.into());
        }
    }
    let looks_float = digits.bytes().any(|b| b.is_ascii_digit())
        && digits.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'-' | b'+'))
        && (digits.contains('.') || digits.contains(['e', 'E']))
        && !digits.starts_with(['e', 'E']);
    if looks_float {
        if let Some(n) = t.parse::<f64>().ok().and_then(Number::from_f64) {
            return Value::Number(n);
        }
    }
    Value::String(t.to_string())
}

// ----- Writing ---------------------------------------------------------------------------------

/// Block-style YAML, as SnakeYAML's default dumper writes EssentialsX's data files.
pub fn to_string(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Object(m) if m.is_empty() => out.push_str("{}\n"),
        Value::Array(a) if a.is_empty() => out.push_str("[]\n"),
        Value::Object(_) | Value::Array(_) => write_block(&mut out, value, 0),
        other => {
            out.push_str(&inline(other));
            out.push('\n');
        }
    }
    out
}

fn write_block(out: &mut String, value: &Value, indent: usize) {
    let pad = " ".repeat(indent);
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                out.push_str(&pad);
                out.push_str(&quote_if_needed(k));
                out.push(':');
                match v {
                    Value::Object(m) if !m.is_empty() => {
                        out.push('\n');
                        write_block(out, v, indent + 2);
                    }
                    Value::Array(a) if !a.is_empty() => {
                        out.push('\n');
                        write_block(out, v, indent);
                    }
                    other => {
                        out.push(' ');
                        out.push_str(&inline(other));
                        out.push('\n');
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                out.push_str(&pad);
                out.push('-');
                match item {
                    Value::Object(m) if !m.is_empty() => {
                        // `- key: v` with the rest of the map aligned after the dash.
                        let mut nested = String::new();
                        write_block(&mut nested, item, indent + 2);
                        out.push(' ');
                        out.push_str(&nested[indent + 2..]);
                    }
                    Value::Array(a) if !a.is_empty() => {
                        out.push('\n');
                        write_block(out, item, indent + 2);
                    }
                    other => {
                        out.push(' ');
                        out.push_str(&inline(other));
                        out.push('\n');
                    }
                }
            }
        }
        other => {
            out.push_str(&pad);
            out.push_str(&inline(other));
            out.push('\n');
        }
    }
}

fn inline(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => quote_if_needed(s),
        Value::Array(a) => format!("[{}]", a.iter().map(inline).collect::<Vec<_>>().join(", ")),
        Value::Object(m) => format!(
            "{{{}}}",
            m.iter().map(|(k, v)| format!("{}: {}", quote_if_needed(k), inline(v))).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// A string written plain when it reads back as the same string, else single-quoted.
fn quote_if_needed(s: &str) -> String {
    let plain_ok = !s.is_empty()
        && s.trim() == s
        && !s.contains(['\n', '\r', '\t'])
        && !s.starts_with(['-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`'])
        && !s.contains(": ")
        && !s.contains(" #")
        && !s.ends_with(':')
        && scalar(s) == Value::String(s.to_string());
    if plain_ok {
        s.to_string()
    } else if s.contains(['\n', '\r', '\t']) {
        serde_json::to_string(s).unwrap_or_default()
    } else {
        format!("'{}'", s.replace('\'', "''"))
    }
}

// ----- Paths -------------------------------------------------------------------------------------

/// `a.b.c` inside nested maps (Bukkit's ConfigurationSection paths).
pub fn get<'v>(root: &'v Value, path: &str) -> Option<&'v Value> {
    let mut at = root;
    for part in path.split('.') {
        at = at.as_object()?.get(part)?;
    }
    Some(at)
}

/// Sets `a.b.c`, creating maps on the way (and replacing non-map values in the way).
pub fn set(root: &mut Value, path: &str, value: Value) {
    let mut at = root;
    let parts: Vec<&str> = path.split('.').collect();
    for (i, part) in parts.iter().enumerate() {
        if !at.is_object() {
            *at = Value::Object(Map::new());
        }
        let map = at.as_object_mut().expect("just made an object");
        if i + 1 == parts.len() {
            if value.is_null() {
                map.remove(*part);
            } else {
                map.insert((*part).to_string(), value);
            }
            return;
        }
        at = map.entry((*part).to_string()).or_insert_with(|| Value::Object(Map::new()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_sequences_and_scalars() {
        let v = parse(
            "# comment\nname: Essentials\nversion: 2.21\nauthors: [a, 'b c', \"d\"]\nenabled: true\nnothing:\ncommands:\n  home:\n    description: Teleport to your home. # trailing\n    usage: /<command> [player:][name]\n    aliases: [ehome, homes]\nlist:\n- one\n- two\n-   three: 3\n    four: 4\n",
        )
        .unwrap();
        assert_eq!(v["name"], "Essentials");
        assert_eq!(v["version"], json!(2.21));
        assert_eq!(v["authors"], json!(["a", "b c", "d"]));
        assert_eq!(v["enabled"], true);
        assert_eq!(v["nothing"], Value::Null);
        assert_eq!(v["commands"]["home"]["usage"], "/<command> [player:][name]");
        assert_eq!(v["commands"]["home"]["description"], "Teleport to your home.");
        assert_eq!(v["commands"]["home"]["aliases"], json!(["ehome", "homes"]));
        assert_eq!(v["list"], json!(["one", "two", {"three": 3, "four": 4}]));
    }

    #[test]
    fn nested_sequences_of_maps_and_block_scalars() {
        let v = parse(
            "kits:\n  tools:\n    delay: 10\n    items:\n      - stone_sword 1\n      - stone_pickaxe 1 efficiency:1\nhomes:\n  - name: base\n    x: 1.5\n  - name: farm\n    x: -2\ntext: |\n  line one\n  line two\nfolded: >\n  a\n  b\n",
        )
        .unwrap();
        assert_eq!(v["kits"]["tools"]["items"], json!(["stone_sword 1", "stone_pickaxe 1 efficiency:1"]));
        assert_eq!(v["homes"], json!([{"name": "base", "x": 1.5}, {"name": "farm", "x": -2}]));
        assert_eq!(v["text"], "line one\nline two\n");
        assert_eq!(v["folded"], "a b\n");
    }

    #[test]
    fn quoting_and_typing() {
        let v = parse("a: 'it''s'\nb: \"tab\\there\"\nc: '#notcomment'\nd: yes\ne: 007\nf: 1e3\ng: '5'\nh: -12\ni: hello: world\n");
        let v = match v {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(v["a"], "it's");
        assert_eq!(v["b"], "tab\there");
        assert_eq!(v["c"], "#notcomment");
        assert_eq!(v["d"], true);
        assert_eq!(v["e"], 7);
        assert_eq!(v["f"], json!(1000.0));
        assert_eq!(v["g"], "5");
        assert_eq!(v["h"], -12);
        assert_eq!(v["i"], "hello: world");
    }

    #[test]
    fn writes_what_it_reads() {
        let value = json!({
            "money": 12.5,
            "homes": {"base": {"world": "minecraft:overworld", "x": 1.5, "y": 64, "yaw": 90.0}},
            "mail": [{"sender": "Bob", "message": "hi: there", "read": false}],
            "ignore": [],
            "nickname": "~Neo",
            "quoted": "yes",
            "empty": ""
        });
        let text = to_string(&value);
        assert_eq!(parse(&text).unwrap(), value, "{text}");
    }

    #[test]
    fn paths() {
        let mut v = json!({});
        set(&mut v, "teleport.delay", json!(3));
        set(&mut v, "teleport.cooldown", json!(0));
        assert_eq!(get(&v, "teleport.delay"), Some(&json!(3)));
        set(&mut v, "teleport.delay", Value::Null);
        assert_eq!(v, json!({"teleport": {"cooldown": 0}}));
    }
}
