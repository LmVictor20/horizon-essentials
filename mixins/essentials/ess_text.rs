// horizon-essentials: chat text. EssentialsX writes its messages in MiniMessage (`<primary>Teleporting...`)
// with java.text.MessageFormat placeholders (`{0}`), and players type legacy `&`/`§` codes. This module turns
// those into styled spans; the game glue turns spans into Horizon's chat components.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    /// A vanilla color name (`gold`) or `#rrggbb`.
    pub color: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
    /// (action, value): `run_command`, `suggest_command`, `open_url`, `copy_to_clipboard`.
    pub click: Option<(String, String)>,
    /// Hover text (plain, already rendered).
    pub hover: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

pub const COLOR_NAMES: [&str; 16] = [
    "black", "dark_blue", "dark_green", "dark_aqua", "dark_red", "dark_purple", "gold", "gray",
    "dark_gray", "blue", "green", "aqua", "red", "light_purple", "yellow", "white",
];
const LEGACY: &str = "0123456789abcdef";

/// The two configurable message colors (config.yml `message-colors`).
#[derive(Clone, Debug)]
pub struct Palette {
    pub primary: String,
    pub secondary: String,
}

impl Default for Palette {
    fn default() -> Self {
        Self { primary: "#ffaa00".into(), secondary: "#ff5555".into() }
    }
}

/// The plain text of spans (what tests usually compare).
pub fn plain(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

fn color_tag(name: &str, palette: &Palette) -> Option<String> {
    let name = name.to_ascii_lowercase();
    match name.as_str() {
        "primary" => return Some(palette.primary.clone()),
        "secondary" => return Some(palette.secondary.clone()),
        "grey" => return Some("gray".into()),
        "dark_grey" => return Some("dark_gray".into()),
        _ => {}
    }
    if COLOR_NAMES.contains(&name.as_str()) {
        return Some(name);
    }
    let hex = name.strip_prefix('#')?;
    (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| format!("#{hex}"))
}

/// The argument of a `<tag:arg>` with MiniMessage's quoting removed.
fn tag_arg(raw: &str) -> String {
    let t = raw.trim();
    if t.len() >= 2 && ((t.starts_with('\'') && t.ends_with('\'')) || (t.starts_with('"') && t.ends_with('"'))) {
        t[1..t.len() - 1].replace("\\'", "'").replace("\\\"", "\"")
    } else {
        t.to_string()
    }
}

/// Finds the `>` that closes a tag opened at `start` (quotes may hold `>`).
fn tag_end(s: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    let bytes = s.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() {
        let c = bytes[i];
        match quote {
            Some(_) if c == b'\\' => i += 1,
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == b'\'' || c == b'"' => quote = Some(c),
            None if c == b'>' => return Some(i),
            None if c == b'<' => return None,
            None => {}
        }
        i += 1;
    }
    None
}

/// MiniMessage as EssentialsX's messages use it. Unknown tags (`<command>` in usages) stay literal text;
/// `\<` is a literal `<`.
pub fn parse_mini(s: &str, palette: &Palette) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut stack: Vec<(String, Style)> = vec![(String::new(), Style::default())];
    let mut text = String::new();
    let current = |stack: &Vec<(String, Style)>| stack.last().map(|(_, s)| s.clone()).unwrap_or_default();
    let flush = |text: &mut String, spans: &mut Vec<Span>, style: Style| {
        if text.is_empty() {
            return;
        }
        match spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            _ => spans.push(Span { text: std::mem::take(text), style }),
        }
        text.clear();
    };
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with("\\<") {
            text.push('<');
            i += 2;
            continue;
        }
        if rest.starts_with('<') {
            if let Some(end) = tag_end(s, i) {
                let inner = &s[i + 1..end];
                let (closing, body) = match inner.strip_prefix('/') {
                    Some(b) => (true, b),
                    None => (false, inner),
                };
                let (name, arg) = match body.split_once(':') {
                    Some((n, a)) => (n.to_ascii_lowercase(), Some(a)),
                    None => (body.to_ascii_lowercase(), None),
                };
                let mut style = current(&stack);
                let known = if closing {
                    let known = name == "reset"
                        || stack.iter().any(|(t, _)| t == &name)
                        || matches!(name.as_str(), "b" | "bold" | "i" | "italic" | "em" | "u" | "underlined" | "st" | "strikethrough" | "obf" | "obfuscated" | "color" | "colour" | "c" | "click" | "hover")
                        || color_tag(&name, &Palette::default()).is_some();
                    if known {
                        flush(&mut text, &mut spans, current(&stack));
                        if let Some(pos) = stack.iter().rposition(|(t, _)| t == &name) {
                            stack.truncate(pos.max(1));
                        }
                    }
                    known
                } else {
                    let mut ok = true;
                    match name.as_str() {
                        "reset" => {
                            flush(&mut text, &mut spans, current(&stack));
                            stack.truncate(1);
                            i = end + 1;
                            continue;
                        }
                        "b" | "bold" => style.bold = true,
                        "i" | "italic" | "em" => style.italic = true,
                        "u" | "underlined" => style.underlined = true,
                        "st" | "strikethrough" => style.strikethrough = true,
                        "obf" | "obfuscated" => style.obfuscated = true,
                        "color" | "colour" | "c" => match arg.and_then(|a| color_tag(&tag_arg(a), palette)) {
                            Some(c) => style.color = Some(c),
                            None => ok = false,
                        },
                        "click" => match arg.and_then(|a| a.split_once(':')) {
                            Some((action, value)) => style.click = Some((action.to_ascii_lowercase(), tag_arg(value))),
                            None => ok = false,
                        },
                        "hover" => match arg.and_then(|a| a.split_once(':')) {
                            Some((_, value)) => style.hover = Some(plain(&parse_mini(&tag_arg(value), palette))),
                            None => ok = false,
                        },
                        other => match color_tag(other, palette) {
                            Some(c) if arg.is_none() => {
                                // A color tag starts fresh decorations, as legacy codes do.
                                style.color = Some(c);
                            }
                            _ => ok = false,
                        },
                    }
                    if ok {
                        flush(&mut text, &mut spans, current(&stack));
                        stack.push((name.clone(), style));
                    }
                    ok
                };
                if known {
                    i = end + 1;
                    continue;
                }
            }
        }
        let ch = rest.chars().next().expect("non-empty");
        text.push(ch);
        i += ch.len_utf8();
    }
    flush(&mut text, &mut spans, current(&stack));
    spans
}

/// `<` in text that must not become a tag (EssentialsX's escapeTags).
pub fn escape_tags(s: &str) -> String {
    s.replace('<', "\\<")
}

/// Legacy `§` codes (and `&` codes when `ampersand`) as MiniMessage tags (EssentialsX's legacyToMini).
/// `§x§r§r§g§g§b§b` hex is supported.
pub fn legacy_to_mini(s: &str, ampersand: bool) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let is_code = |c: char| c == '§' || (ampersand && c == '&');
    while i < chars.len() {
        let c = chars[i];
        if is_code(c) && i + 1 < chars.len() {
            let code = chars[i + 1].to_ascii_lowercase();
            if code == 'x' && i + 13 < chars.len() && (0..6).all(|k| is_code(chars[i + 2 + k * 2]) && chars[i + 3 + k * 2].is_ascii_hexdigit()) {
                let hex: String = (0..6).map(|k| chars[i + 3 + k * 2]).collect();
                out.push_str(&format!("<#{}>", hex.to_ascii_lowercase()));
                i += 14;
                continue;
            }
            if let Some(n) = LEGACY.find(code) {
                out.push_str("<reset><");
                out.push_str(COLOR_NAMES[n]);
                out.push('>');
                i += 2;
                continue;
            }
            let tag = match code {
                'k' => Some("obfuscated"),
                'l' => Some("bold"),
                'm' => Some("strikethrough"),
                'n' => Some("underlined"),
                'o' => Some("italic"),
                'r' => Some("reset"),
                _ => None,
            };
            if let Some(tag) = tag {
                out.push('<');
                out.push_str(tag);
                out.push('>');
                i += 2;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Strips legacy codes (`§` and, with `ampersand`, `&`).
pub fn strip_legacy(s: &str, ampersand: bool) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == '§' || (ampersand && c == '&')) && i + 1 < chars.len() && "0123456789abcdefklmnorx".contains(chars[i + 1].to_ascii_lowercase()) {
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// One argument of a message: plain text is escaped (and its legacy codes kept as colors), parsed text is
/// MiniMessage inserted as is (AdventureUtil.parsed).
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Plain(String),
    Parsed(String),
}

impl Arg {
    fn render(&self) -> String {
        match self {
            Arg::Plain(s) => legacy_to_mini(&escape_tags(s), false),
            Arg::Parsed(s) => s.clone(),
        }
    }
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Plain(s.to_string())
    }
}
impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Plain(s)
    }
}
impl From<&String> for Arg {
    fn from(s: &String) -> Self {
        Arg::Plain(s.clone())
    }
}
macro_rules! arg_from_number {
    ($($t:ty),*) => {$(impl From<$t> for Arg {
        fn from(n: $t) -> Self { Arg::Plain(group_digits(&n.to_string())) }
    })*};
}
arg_from_number!(i32, i64, u32, u64, usize);

/// "1234567" as MessageFormat writes a Number in English: "1,234,567".
pub fn group_digits(n: &str) -> String {
    let (sign, digits) = n.strip_prefix('-').map_or(("", n), |d| ("-", d));
    let (int, frac) = digits.split_once('.').map_or((digits, None), |(a, b)| (a, Some(b)));
    let mut out = String::new();
    for (k, ch) in int.chars().enumerate() {
        if k > 0 && (int.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    match frac {
        Some(f) => format!("{sign}{out}.{f}"),
        None => format!("{sign}{out}"),
    }
}

/// java.text.MessageFormat as EssentialsX uses it: `{n}` placeholders, `''` for a quote, `'...'` literal.
/// Without arguments EssentialsX skips MessageFormat and only turns `''` into `'`.
pub fn message_format(pattern: &str, args: &[Arg]) -> String {
    if args.is_empty() {
        return pattern.replace("''", "'");
    }
    let mut out = String::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            if chars.get(i + 1) == Some(&'\'') {
                out.push('\'');
                i += 2;
                continue;
            }
            quoted = !quoted;
            i += 1;
            continue;
        }
        if c == '{' && !quoted {
            if let Some(close) = chars[i..].iter().position(|&ch| ch == '}') {
                let inner: String = chars[i + 1..i + close].iter().collect();
                let index = inner.split(',').next().unwrap_or("").trim();
                if let Ok(n) = index.parse::<usize>() {
                    match args.get(n) {
                        Some(a) => out.push_str(&a.render()),
                        None => {
                            out.push('{');
                            out.push_str(index);
                            out.push('}');
                        }
                    }
                    i += close + 1;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out.replace('\u{a0}', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Palette {
        Palette::default()
    }

    #[test]
    fn essentials_messages_render() {
        let spans = parse_mini("<primary>Request sent to<secondary> Bob<primary>.", &p());
        assert_eq!(plain(&spans), "Request sent to Bob.");
        assert_eq!(spans[0].style.color.as_deref(), Some("#ffaa00"));
        assert_eq!(spans[1].text, " Bob");
        assert_eq!(spans[1].style.color.as_deref(), Some("#ff5555"));
    }

    #[test]
    fn unknown_tags_stay_text_and_escapes_work() {
        let spans = parse_mini("<dark_red>/<command> <player> \\<b>", &p());
        assert_eq!(plain(&spans), "/<command> <player> <b>");
        assert_eq!(spans[0].style.color.as_deref(), Some("dark_red"));
    }

    #[test]
    fn decorations_close_and_reset() {
        let spans = parse_mini("<green><b>Bold</b> plain<reset> none", &p());
        assert_eq!(spans.len(), 3);
        assert!(spans[0].style.bold && spans[0].style.color.as_deref() == Some("green"));
        assert!(!spans[1].style.bold && spans[1].style.color.as_deref() == Some("green"));
        assert_eq!(spans[2].style, Style::default());
    }

    #[test]
    fn click_and_hover() {
        let spans = parse_mini("<click:open_url:\"https://x.y\"><hover:show_text:'<red>Open'>link</hover></click>", &p());
        assert_eq!(spans[0].text, "link");
        assert_eq!(spans[0].style.click, Some(("open_url".into(), "https://x.y".into())));
        assert_eq!(spans[0].style.hover.as_deref(), Some("Open"));
    }

    #[test]
    fn message_format_matches_java() {
        assert_eq!(message_format("Can''t find {0}!", &["db".into()]), "Can't find db!");
        assert_eq!(message_format("Can''t", &[]), "Can't");
        assert_eq!(message_format("'{0}' is {0}", &["x".into()]), "{0} is x");
        assert_eq!(message_format("{1} {0} {2}", &["a".into(), "b".into()]), "b a {2}");
        assert_eq!(message_format("{0}", &[1234567i64.into()]), "1,234,567");
    }

    #[test]
    fn plain_args_cannot_inject_tags_but_keep_legacy_colors() {
        let msg = message_format("<primary>{0}", &["<red>§ahi".into()]);
        assert_eq!(msg, "<primary>\\<red><reset><green>hi");
        let spans = parse_mini(&msg, &p());
        assert_eq!(plain(&spans), "<red>hi");
        assert_eq!(spans.last().unwrap().style.color.as_deref(), Some("green"));
        assert_eq!(message_format("{0}", &[Arg::Parsed("<red>x".into())]), "<red>x");
    }

    #[test]
    fn legacy_codes() {
        assert_eq!(legacy_to_mini("&cHi &lthere", true), "<reset><red>Hi <bold>there");
        assert_eq!(legacy_to_mini("&cHi", false), "&cHi");
        assert_eq!(legacy_to_mini("§x§f§f§0§0§0§0red", false), "<#ff0000>red");
        assert_eq!(strip_legacy("&cHi §lthere", true), "Hi there");
    }
}
