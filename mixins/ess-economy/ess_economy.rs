// horizon-essentials area `economy` (pure logic, standalone-tested): balances (userdata `money`), /balance,
// /balancetop, /eco, /pay, /payconfirmtoggle, /paytoggle, /sell, /setworth, /worth, command costs, the economy
// log and every Essentials sign. Ported from EssentialsX (GPL-3.0): commands/Command{balance,balancetop,eco,pay,
// payconfirmtoggle,paytoggle,sell,setworth,worth}.java, User/UserData money, Worth.java, Trade.java,
// utils/NumberUtil.java, BalanceTopImpl.java and signs/*.java.
//
// Money is an exact decimal (`Dec`, BigDecimal's semantics: value and scale), never a float. Signs are read and
// written as legacy text (`§1[Buy]`), as Bukkit's Sign API gives them; the glue stores those strings as the
// sign's plain text components, which vanilla clients render with their legacy colours.

use crate::game::ess_core::*;
use crate::game::{ess_text, ess_yaml};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn register(ess: &mut Ess) {
    ess.register(Command {
        name: "balance",
        run: cmd_balance,
        tab: None,
        usages: &[("/<command>", "balanceCommandUsage1Description"), ("/<command> <player>", "balanceCommandUsage2Description")],
    });
    ess.register(Command { name: "balancetop", run: cmd_balancetop, tab: None, usages: &[("/<command> [page]", "balancetopCommandUsage1Description")] });
    ess.register(Command {
        name: "eco",
        run: cmd_eco,
        tab: None,
        usages: &[
            ("/<command> give <player> <amount>", "ecoCommandUsage1Description"),
            ("/<command> take <player> <amount>", "ecoCommandUsage2Description"),
            ("/<command> set <player> <amount>", "ecoCommandUsage3Description"),
            ("/<command> reset <player> <amount>", "ecoCommandUsage4Description"),
        ],
    });
    ess.register(Command { name: "pay", run: cmd_pay, tab: None, usages: &[("/<command> <player> <amount>", "payCommandUsage1Description")] });
    ess.register(Command { name: "payconfirmtoggle", run: cmd_payconfirmtoggle, tab: None, usages: &[] });
    ess.register(Command { name: "paytoggle", run: cmd_paytoggle, tab: None, usages: &[("/<command> [player]", "paytoggleCommandUsage1Description")] });
    ess.register(Command {
        name: "sell",
        run: cmd_sell,
        tab: None,
        usages: &[
            ("/<command> <itemname> [amount]", "sellCommandUsage1Description"),
            ("/<command> hand [amount]", "sellCommandUsage2Description"),
            ("/<command> all", "sellCommandUsage3Description"),
            ("/<command> blocks [amount]", "sellCommandUsage4Description"),
        ],
    });
    ess.register(Command {
        name: "setworth",
        run: cmd_setworth,
        tab: None,
        usages: &[("/<command> <price>", "setworthCommandUsage1Description"), ("/<command> <itemname> <price>", "setworthCommandUsage2Description")],
    });
    ess.register(Command {
        name: "worth",
        run: cmd_worth,
        tab: None,
        usages: &[
            ("/<command> <itemname> [amount]", "worthCommandUsage1Description"),
            ("/<command> hand [amount]", "worthCommandUsage2Description"),
            ("/<command> all", "worthCommandUsage3Description"),
            ("/<command> blocks [amount]", "worthCommandUsage4Description"),
        ],
    });
    ess.listeners.command.push(on_command_cost);
    ess.listeners.sign_change.push(on_sign_change);
    ess.listeners.interact.push(on_interact);
    ess.listeners.block_break.push(on_block_break);
    ess.listeners.block_place.push(on_block_place);
    ess.listeners.world.push(on_world);
}

// ===== Exact decimals ==============================================================================

/// An exact decimal: `m × 10^-s` (java.math.BigDecimal: equality includes the scale, `cmp` compares values).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dec {
    m: i128,
    s: u32,
}

fn pow10(n: u32) -> Option<i128> {
    10i128.checked_pow(n)
}

impl Dec {
    pub const ZERO: Dec = Dec { m: 0, s: 0 };
    pub const ONE: Dec = Dec { m: 1, s: 0 };

    pub fn from_i64(n: i64) -> Dec {
        Dec { m: n as i128, s: 0 }
    }
    /// BigDecimal(String): `-12.50`, `.5`, `5.`, `1e3`, `1.5E-2`.
    pub fn parse(text: &str) -> Option<Dec> {
        let t = text.trim();
        let (neg, rest) = match t.as_bytes().first()? {
            b'-' => (true, &t[1..]),
            b'+' => (false, &t[1..]),
            _ => (false, t),
        };
        let (num, exp) = match rest.find(['e', 'E']) {
            Some(i) => (&rest[..i], rest[i + 1..].parse::<i32>().ok()?),
            None => (rest, 0),
        };
        let (int, frac) = num.split_once('.').unwrap_or((num, ""));
        if int.is_empty() && frac.is_empty() {
            return None;
        }
        if !int.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let digits = format!("{int}{frac}");
        let digits = digits.trim_start_matches('0');
        let mut m: i128 = if digits.is_empty() { 0 } else { digits.parse().ok()? };
        let mut scale = frac.len() as i64 - exp as i64;
        if scale < 0 {
            m = m.checked_mul(pow10((-scale) as u32)?)?;
            scale = 0;
        }
        let s = u32::try_from(scale).ok()?;
        Some(Dec { m: if neg { -m } else { m }, s })
    }
    /// A config or userdata value (number or string).
    pub fn from_value(v: &Value) -> Option<Dec> {
        match v {
            Value::String(s) => Dec::parse(s),
            Value::Number(n) => Dec::parse(&n.to_string()),
            _ => None,
        }
    }
    fn rescale(&self, s: u32) -> Option<i128> {
        if s >= self.s { self.m.checked_mul(pow10(s - self.s)?) } else { Some(self.m / pow10(self.s - s)?) }
    }
    fn aligned(&self, o: &Dec) -> (i128, i128, u32) {
        let s = self.s.max(o.s);
        (self.rescale(s).unwrap_or(i128::MAX), o.rescale(s).unwrap_or(i128::MAX), s)
    }
    pub fn add(&self, o: &Dec) -> Dec {
        let (a, b, s) = self.aligned(o);
        Dec { m: a.saturating_add(b), s }
    }
    pub fn sub(&self, o: &Dec) -> Dec {
        let (a, b, s) = self.aligned(o);
        Dec { m: a.saturating_sub(b), s }
    }
    pub fn mul(&self, o: &Dec) -> Dec {
        match self.m.checked_mul(o.m) {
            Some(m) => Dec { m, s: self.s + o.s },
            None => Dec { m: self.m.saturating_mul(o.m.signum()), s: self.s }.mul_slow(o),
        }
    }
    fn mul_slow(&self, _o: &Dec) -> Dec {
        // Out of range for 128 bits: saturate (no balance gets near this; max-money is far smaller).
        Dec { m: self.m, s: 0 }
    }
    /// BigDecimal.divide(divisor): the exact quotient, or None when it doesn't terminate (ArithmeticException).
    pub fn div_exact(&self, o: &Dec) -> Option<Dec> {
        if o.m == 0 {
            return None;
        }
        let preferred = self.s as i64 - o.s as i64;
        for k in 0..=38u32 {
            let n = self.m.checked_mul(pow10(k)?)?;
            if n % o.m == 0 {
                let q = n / o.m;
                let scale = preferred + k as i64;
                return if scale < 0 { Some(Dec { m: q.checked_mul(pow10((-scale) as u32)?)?, s: 0 }) } else { Some(Dec { m: q, s: scale as u32 }) };
            }
        }
        None
    }
    /// BigDecimal.remainder: `this - this.divideToIntegralValue(o) * o`.
    pub fn rem(&self, o: &Dec) -> Dec {
        let (a, b, s) = self.aligned(o);
        if b == 0 {
            return self.clone();
        }
        Dec { m: a % b, s }
    }
    pub fn cmp(&self, o: &Dec) -> Ordering {
        let (a, b, _) = self.aligned(o);
        a.cmp(&b)
    }
    pub fn signum(&self) -> i32 {
        self.m.signum() as i32
    }
    pub fn neg(&self) -> Dec {
        Dec { m: -self.m, s: self.s }
    }
    pub fn abs(&self) -> Dec {
        Dec { m: self.m.abs(), s: self.s }
    }
    /// Multiplied by 10^n (n may be negative), as BigDecimal.scaleByPowerOfTen.
    pub fn scale_by_pow10(&self, n: i32) -> Dec {
        if n >= 0 {
            match self.s.checked_sub(n as u32) {
                Some(s) => Dec { m: self.m, s },
                None => Dec { m: self.m.saturating_mul(pow10(n as u32 - self.s).unwrap_or(i128::MAX)), s: 0 },
            }
        } else {
            Dec { m: self.m, s: self.s + (-n) as u32 }
        }
    }
    /// BigDecimal.stripTrailingZeros (scale kept at 0 or more).
    pub fn strip_trailing_zeros(&self) -> Dec {
        let (mut m, mut s) = (self.m, self.s);
        while s > 0 && m % 10 == 0 {
            m /= 10;
            s -= 1;
        }
        if m == 0 {
            s = 0;
        }
        Dec { m, s }
    }
    /// Rounded to `scale` digits towards negative infinity (RoundingMode.FLOOR).
    pub fn floor_to(&self, scale: u32) -> Dec {
        if scale >= self.s {
            return Dec { m: self.rescale(scale).unwrap_or(self.m), s: scale };
        }
        let p = pow10(self.s - scale).unwrap_or(1);
        Dec { m: self.m.div_euclid(p), s: scale }
    }
    /// The whole part (truncated towards zero) as i64, saturating.
    pub fn trunc_i64(&self) -> i64 {
        let p = pow10(self.s).unwrap_or(1);
        (self.m / p).clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }
    pub fn to_f64(&self) -> f64 {
        self.to_string().parse().unwrap_or(0.0)
    }
    /// (sign, integer digits, fraction digits) of the plain form.
    fn parts(&self) -> (bool, String, String) {
        let digits = self.m.unsigned_abs().to_string();
        let s = self.s as usize;
        let (int, frac) = if digits.len() > s {
            (digits[..digits.len() - s].to_string(), digits[digits.len() - s..].to_string())
        } else {
            ("0".to_string(), format!("{}{}", "0".repeat(s - digits.len()), digits))
        };
        (self.m < 0, int, frac)
    }
}

impl std::fmt::Display for Dec {
    /// BigDecimal.toPlainString.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (neg, int, frac) = self.parts();
        let sign = if neg { "-" } else { "" };
        if frac.is_empty() { write!(f, "{sign}{int}") } else { write!(f, "{sign}{int}.{frac}") }
    }
}

/// NumberUtil.parseStringToBDecimal: digits with `,` grouping and `.` decimals, then an optional K/M/B/T
/// multiplier. As java.text.NumberFormat returns a Long or a Double, a whole number has scale 0 and a fraction
/// loses its trailing zeros (`10.50` is 10.5).
pub fn parse_amount(arg: &str) -> Option<Dec> {
    if arg.is_empty() {
        return None;
    }
    let sanitized: String = arg.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    let modifier = if sanitized.is_empty() { arg.to_uppercase() } else { arg.replace(&sanitized, "").to_uppercase() };
    let multiplier = match modifier.as_str() {
        "" => None,
        "K" => Some(1_000i64),
        "M" => Some(1_000_000),
        "B" => Some(1_000_000_000),
        "T" => Some(1_000_000_000_000),
        _ => return None,
    };
    // DecimalFormat.parse in the US locale: grouping commas in the integer part, one decimal point.
    let mut int = String::new();
    let mut frac = String::new();
    let mut in_frac = false;
    for c in sanitized.chars() {
        match c {
            '0'..='9' if in_frac => frac.push(c),
            '0'..='9' => int.push(c),
            ',' if !in_frac => {}
            '.' if !in_frac => in_frac = true,
            _ => break,
        }
    }
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    let parsed = Dec::parse(&format!("{}.{}", if int.is_empty() { "0" } else { &int }, if frac.is_empty() { "0" } else { &frac }))?;
    let mut amount = parsed.strip_trailing_zeros();
    if amount.s > 0 {
        // Double.toString keeps the fraction digits; a whole Double would have been a Long.
    } else {
        amount = Dec { m: amount.m, s: 0 };
    }
    if let Some(m) = multiplier {
        amount = amount.mul(&Dec::from_i64(m));
    }
    Some(amount)
}

// ===== Settings ====================================================================================

fn config_dec(ess: &Ess, path: &str, default: &str) -> Dec {
    ess.config(path).and_then(Dec::from_value).unwrap_or_else(|| Dec::parse(default).expect("default decimal"))
}
fn starting_balance(ess: &Ess) -> Dec {
    config_dec(ess, "starting-balance", "0")
}
fn max_money(ess: &Ess) -> Dec {
    config_dec(ess, "max-money", "10000000000000")
}
fn min_money(ess: &Ess) -> Dec {
    let min = config_dec(ess, "min-money", "-10000000000000");
    if min.signum() > 0 { min.neg() } else { min }
}
fn eco_disabled(ess: &Ess) -> bool {
    ess.config_bool("disable-eco", false)
}
/// Settings._getCurrencySymbol: one non-digit character, else `$`.
fn currency_symbol(ess: &Ess) -> String {
    let value = ess.config_str("currency-symbol").unwrap_or_else(|| "$".into()).trim().to_string();
    if value.chars().count() > 1 || value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty() {
        return "$".into();
    }
    value
}
fn symbol_suffixed(ess: &Ess) -> bool {
    ess.config_bool("currency-symbol-suffix", false)
}
fn minimum_pay_amount(ess: &Ess) -> Dec {
    config_dec(ess, "minimum-pay-amount", "0.001")
}

// ===== Currency formatting (NumberUtil) ====================================================================

/// (grouping separator, decimal separator) of `currency-symbol-format-locale` (DecimalFormatSymbols).
fn locale_symbols(ess: &Ess) -> (char, char) {
    let tag = ess.config_str("currency-symbol-format-locale").unwrap_or_default().replace('_', "-");
    let lower = tag.to_lowercase();
    let lang = lower.split('-').next().unwrap_or("");
    if lower == "de-ch" || lower == "it-ch" || lower == "fr-ch" {
        return ('\u{2019}', '.');
    }
    match lang {
        "de" | "es" | "it" | "nl" | "pt" | "id" | "da" | "tr" | "el" | "ro" | "hr" | "sl" | "sr" | "is" | "vi" => ('.', ','),
        "fr" => ('\u{202f}', ','),
        "ru" | "uk" | "pl" | "cs" | "sk" | "fi" | "sv" | "nb" | "no" | "hu" | "bg" | "et" | "lv" | "lt" => ('\u{a0}', ','),
        _ => (',', '.'),
    }
}

/// java.text.DecimalFormat with RoundingMode.FLOOR for the usual patterns (`#,##0.00`, `#0.00`, `#,##0.###`).
fn format_decimal(value: &Dec, pattern: &str, symbols: (char, char)) -> String {
    let pattern = pattern.split(';').next().unwrap_or(pattern);
    let body: String = pattern.chars().filter(|c| matches!(c, '#' | '0' | ',' | '.')).collect();
    let (int_pat, frac_pat) = body.split_once('.').unwrap_or((&body, ""));
    let grouping = int_pat.rfind(',').map(|i| int_pat.len() - i - 1).filter(|g| *g > 0);
    let min_int = int_pat.chars().filter(|c| *c == '0').count();
    let min_frac = frac_pat.chars().filter(|c| *c == '0').count();
    let max_frac = frac_pat.chars().filter(|c| *c == '0' || *c == '#').count();
    let rounded = value.floor_to(max_frac as u32);
    let (neg, int, frac) = rounded.parts();
    let mut int = int.trim_start_matches('0').to_string();
    while int.len() < min_int {
        int.insert(0, '0');
    }
    let mut frac = frac;
    while frac.len() > min_frac && frac.ends_with('0') {
        frac.pop();
    }
    let mut grouped = String::new();
    if let Some(g) = grouping {
        let chars: Vec<char> = int.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % g == 0 {
                grouped.push(symbols.0);
            }
            grouped.push(*c);
        }
    } else {
        grouped = int;
    }
    let zero = rounded.m == 0;
    let mut out = String::new();
    if neg && !zero {
        out.push('-');
    }
    out.push_str(&grouped);
    if !frac.is_empty() {
        out.push(symbols.1);
        out.push_str(&frac);
    }
    out
}

/// NumberUtil.formatAsCurrency: `#0.00` (US, FLOOR) without a trailing `.00`.
pub fn format_as_currency(value: &Dec) -> String {
    let s = format_decimal(value, "#0.00", (',', '.'));
    s.strip_suffix(".00").map(str::to_string).unwrap_or(s)
}

/// NumberUtil.formatAsPrettyCurrency: `currency-format` without a trailing `.00`.
fn format_as_pretty_currency(ess: &Ess, value: &Dec) -> String {
    let pattern = ess.config_str("currency-format").unwrap_or_else(|| "#,##0.00".into());
    let s = format_decimal(value, &pattern, locale_symbols(ess));
    s.strip_suffix(".00").map(str::to_string).unwrap_or(s)
}

/// NumberUtil.shortCurrency: `$10.5` style for signs.
pub fn short_currency(ess: &Ess, value: &Dec) -> String {
    let symbol = currency_symbol(ess);
    if symbol_suffixed(ess) { format!("{}{symbol}", format_as_currency(value)) } else { format!("{symbol}{}", format_as_currency(value)) }
}

fn display(ess: &Ess, value: &Dec, exact: bool) -> String {
    let mut currency = if exact { value.to_string() } else { format_as_pretty_currency(ess, value) };
    let mut sign = "";
    if value.signum() < 0 {
        currency = currency.chars().skip(1).collect();
        sign = "-";
    }
    let symbol = currency_symbol(ess);
    let text = if symbol_suffixed(ess) {
        ess.tl("currency", &[Arg::Plain(currency), Arg::Plain(symbol)])
    } else {
        ess.tl("currency", &[Arg::Plain(symbol), Arg::Plain(currency)])
    };
    format!("{sign}{text}")
}

/// NumberUtil.displayCurrency: MiniMessage, to be passed as a parsed argument.
pub fn display_currency(ess: &Ess, value: &Dec) -> String {
    display(ess, value, false)
}
/// NumberUtil.displayCurrencyExactly.
pub fn display_currency_exactly(ess: &Ess, value: &Dec) -> String {
    display(ess, value, true)
}
fn cur(ess: &Ess, value: &Dec) -> Arg {
    Arg::Parsed(display_currency(ess, value))
}
/// MiniMessage as legacy `§` text (AdventureFacet.miniToLegacy), for sign lines and plain arguments.
fn mini_to_legacy(ess: &Ess, mini: &str) -> String {
    let mut out = String::new();
    for span in ess.render(mini) {
        let st = &span.style;
        if let Some(c) = &st.color {
            if let Some(i) = ess_text::COLOR_NAMES.iter().position(|n| n == c) {
                out.push('§');
                out.push("0123456789abcdef".as_bytes()[i] as char);
            } else if let Some(hex) = c.strip_prefix('#') {
                out.push_str("§x");
                for ch in hex.chars() {
                    out.push('§');
                    out.push(ch);
                }
            }
        }
        for (on, code) in [(st.obfuscated, 'k'), (st.bold, 'l'), (st.strikethrough, 'm'), (st.underlined, 'n'), (st.italic, 'o')] {
            if on {
                out.push('§');
                out.push(code);
            }
        }
        out.push_str(&span.text);
    }
    out
}

// ===== Balances (User / UserData) ===============================================================================

/// UserData._getMoney: the stored balance (starting-balance when none), kept within min/max-money.
pub fn money(ess: &mut Ess, player: Uuid) -> Dec {
    let mut result = if ess.users.get(player).is_some_and(|u| u.bool("npc")) { Dec::ZERO } else { starting_balance(ess) };
    if let Some(stored) = ess.users.get(player).and_then(|u| u.get("money").and_then(Dec::from_value)) {
        result = stored;
    }
    let (max, min) = (max_money(ess), min_money(ess));
    if result.cmp(&max) == Ordering::Greater {
        result = max;
    }
    if result.cmp(&min) == Ordering::Less {
        result = min;
    }
    result
}

/// User.setMoney: `throw_max` refuses a balance above max-money (MaxMoneyException); below min-money is clamped.
fn set_money(ess: &mut Ess, host: &mut dyn Host, player: Uuid, value: &Dec, throw_max: bool) -> Result<(), CmdError> {
    if eco_disabled(ess) {
        return Ok(());
    }
    let (max, min) = (max_money(ess), min_money(ess));
    let mut value = value.clone();
    if value.cmp(&max) == Ordering::Greater {
        if throw_max {
            return Err(CmdError::Tl("maxMoney", vec![]));
        }
        value = max;
    }
    if value.cmp(&min) == Ordering::Less {
        value = min;
    }
    ess.users.user(player).set("money", json!(value.to_string()));
    let name = user_name(ess, host, player);
    log_trade(ess, host, "Update", "Set", "API", Some(&name), Some(&Trade::Money(value.clone())), None, None, None, Some(&value));
    Ok(())
}

/// User.canAfford: with essentials.eco.loan down to min-money, else down to zero.
fn can_afford(ess: &mut Ess, host: &mut dyn Host, player: Uuid, cost: &Dec) -> bool {
    if cost.signum() <= 0 {
        return true;
    }
    let remaining = money(ess, player).sub(cost);
    if ess.has(host, Sender::Player(player), "essentials.eco.loan") {
        return remaining.cmp(&min_money(ess)) != Ordering::Less;
    }
    remaining.signum() >= 0
}

/// User.giveMoney: "added to your account" (and to the initiator, the new balance).
fn give_money(ess: &mut Ess, host: &mut dyn Host, player: Uuid, value: &Dec, initiator: Option<Sender>) -> Result<(), CmdError> {
    if value.signum() == 0 {
        return Ok(());
    }
    let new = money(ess, player).add(value);
    set_money(ess, host, player, &new, true)?;
    ess.send_tl(host, Sender::Player(player), "addedToAccount", &[cur(ess, value)]);
    if let Some(to) = initiator {
        let name = display_name(ess, host, player);
        let balance = money(ess, player);
        ess.send_tl(host, to, "addedToOthersAccount", &[cur(ess, value), Arg::Plain(name), cur(ess, &balance)]);
    }
    Ok(())
}

/// User.takeMoney: "taken from your account".
fn take_money(ess: &mut Ess, host: &mut dyn Host, player: Uuid, value: &Dec, initiator: Option<Sender>) {
    if value.signum() == 0 {
        return;
    }
    let new = money(ess, player).sub(value);
    let _ = set_money(ess, host, player, &new, false);
    ess.send_tl(host, Sender::Player(player), "takenFromAccount", &[cur(ess, value)]);
    if let Some(to) = initiator {
        let name = display_name(ess, host, player);
        let balance = money(ess, player);
        ess.send_tl(host, to, "takenFromOthersAccount", &[cur(ess, value), Arg::Plain(name), cur(ess, &balance)]);
    }
}

fn user_name(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    host.name(player).or_else(|| ess.users.get(player).and_then(|u| u.name())).unwrap_or_else(|| uuid_string(player))
}
/// User.getDisplayName: the nickname (with nickname-prefix) when one is set, else the name.
fn display_name(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> String {
    let nick = ess.users.get(player).and_then(|u| u.str("nickname")).filter(|n| !n.is_empty());
    match nick {
        Some(n) => format!("{}{n}", ess.config_str("nickname-prefix").unwrap_or_else(|| "~".into())),
        None => user_name(ess, host, player),
    }
}
fn is_online(host: &mut dyn Host, player: Uuid) -> bool {
    host.online().contains(&player)
}

// ===== Economy log (Trade.log) ==================================================================================

fn trade_csv(ess: &Ess, t: Option<&Trade>) -> String {
    match t {
        None => "\"\",\"\",\"\"".into(),
        Some(Trade::Item(i)) => format!("{},{},", i.count, material(&i.id).to_uppercase()),
        Some(Trade::Money(m)) => format!("{m},money,{}", currency_symbol(ess)),
        Some(Trade::Exp(e)) => format!("{e},exp,\"\""),
        Some(Trade::Command(..)) => String::new(),
    }
}

/// Trade.log: one CSV line in `trade.log` when economy-log-enabled (with a location) or
/// economy-log-update-enabled (balance updates, no location).
#[allow(clippy::too_many_arguments)]
fn log_trade(
    ess: &mut Ess,
    host: &mut dyn Host,
    kind: &str,
    subtype: &str,
    event: &str,
    sender: Option<&str>,
    charge: Option<&Trade>,
    receiver: Option<&str>,
    pay: Option<&Trade>,
    loc: Option<(&str, [i32; 3])>,
    end_balance: Option<&Dec>,
) {
    let enabled = if loc.is_none() { ess.config_bool("economy-log-update-enabled", false) } else { ess.config_bool("economy-log-enabled", false) };
    if !enabled {
        return;
    }
    let uuids = ess.config_bool("economy-log-uuids", false);
    let ident = |ess: &Ess, name: &str| -> String {
        if uuids {
            if let Some(u) = ess.users.names.get(&name.to_lowercase()) {
                return uuid_string(*u);
            }
        }
        name.to_string()
    };
    let now = host.now_millis();
    let date = chrono::DateTime::from_timestamp_millis(now)
        .map(|d| d.with_timezone(&chrono::Local).format("%A, %B %-d, %Y at %-I:%M:%S %p %Z").to_string())
        .unwrap_or_default();
    let mut sb = format!("{kind},{subtype},{event},\"{date}\",\"");
    if let Some(s) = sender {
        sb.push_str(&ident(ess, s));
    }
    sb.push_str("\",");
    sb.push_str(&trade_csv(ess, charge));
    sb.push_str(",\"");
    if let Some(r) = receiver {
        sb.push_str(&ident(ess, r));
    }
    sb.push_str("\",");
    sb.push_str(&trade_csv(ess, pay));
    match loc {
        None => sb.push_str(",\"\",\"\",\"\",\"\""),
        Some((world, p)) => sb.push_str(&format!(",\"{world}\",{},{},{},", p[0], p[1], p[2])),
    }
    match end_balance {
        None => sb.push(','),
        Some(b) => sb.push_str(&format!("{b},")),
    }
    sb.push('\n');
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(ess.dir.join("trade.log")) {
        let _ = f.write_all(sb.as_bytes());
    }
}

fn player_block(host: &mut dyn Host, player: Uuid) -> Option<(String, [i32; 3])> {
    host.location(player).map(|l| (l.world.clone(), l.block()))
}

// ===== Items, inventories and worth ==============================================================================

/// `minecraft:iron_ingot` -> `iron_ingot` (Material.toString().toLowerCase()).
fn material(id: &str) -> String {
    id.trim_start_matches("minecraft:").to_string()
}
/// The worth.yml / trade-in-stacks key: `ironingot`.
fn worth_key(id: &str) -> String {
    material(id).replace('_', "")
}
fn similar(a: &Item, b: &Item) -> bool {
    let empty = |v: &Value| v.is_null() || v.as_object().is_some_and(|o| o.is_empty());
    a.id == b.id && (a.components == b.components || empty(&a.components) && empty(&b.components))
}
fn has_custom_name(item: &Item) -> bool {
    item.components.get("custom_name").or_else(|| item.components.get("minecraft:custom_name")).is_some_and(|v| !v.is_null())
}
fn custom_name(item: &Item) -> String {
    match item.components.get("custom_name").or_else(|| item.components.get("minecraft:custom_name")) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.get("text").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| v.to_string()),
        None => String::new(),
    }
}
fn damage(item: &Item) -> i64 {
    item.components.get("damage").or_else(|| item.components.get("minecraft:damage")).and_then(Value::as_i64).unwrap_or(0)
}

/// ItemDb.get: an item by id (`diamond`, `minecraft:iron_ingot`), its underscore-less name (`ironingot`) or a
/// custom_items.yml alias.
fn resolve_item(ess: &Ess, host: &mut dyn Host, name: &str) -> Result<Item, CmdError> {
    let lookup = |host: &mut dyn Host, n: &str| -> Option<String> {
        let n = n.to_lowercase();
        let n = n.trim_start_matches("minecraft:");
        let ids = host.item_ids();
        if ids.is_empty() {
            return (!n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then(|| format!("minecraft:{n}"));
        }
        let full = format!("minecraft:{n}");
        if ids.contains(&full) {
            return Some(full);
        }
        let squashed = n.replace('_', "");
        ids.into_iter().find(|id| worth_key(id) == squashed)
    };
    if let Some(id) = lookup(host, name) {
        return Ok(Item::new(&id, 1));
    }
    let aliases = ess.read_yaml("custom_items.yml");
    if let Some(target) = aliases.get("aliases").and_then(|a| a.get(name.to_lowercase())).and_then(Value::as_str) {
        if let Some(id) = lookup(host, target) {
            return Ok(Item::new(&id, 1));
        }
    }
    Err(CmdError::Tl("unknownItemName", vec![Arg::Plain(name.to_string())]))
}

const STORAGE: std::ops::Range<usize> = 0..36;

fn count_similar(host: &mut dyn Host, player: Uuid, item: &Item) -> i32 {
    host.inventory(player).get(STORAGE).unwrap_or(&[]).iter().flatten().filter(|s| similar(s, item)).map(|s| s.count).sum()
}
fn contains_at_least(host: &mut dyn Host, player: Uuid, item: &Item, amount: i32) -> bool {
    count_similar(host, player, item) >= amount
}
fn remove_items(host: &mut dyn Host, player: Uuid, item: &Item, mut amount: i32) {
    let inv = host.inventory(player);
    for slot in STORAGE {
        if amount <= 0 {
            break;
        }
        if let Some(Some(s)) = inv.get(slot) {
            if similar(s, item) {
                let take = s.count.min(amount);
                amount -= take;
                let mut left = s.clone();
                left.count -= take;
                host.set_slot(player, slot, (left.count > 0).then_some(left));
            }
        }
    }
}
/// Inventories.addItem: partial stacks first, then empty slots; returns what didn't fit.
fn add_items(host: &mut dyn Host, player: Uuid, item: &Item, simulate: bool) -> i32 {
    let max = host.max_stack(&item.id).max(1);
    let mut inv = host.inventory(player);
    let mut left = item.count;
    for slot in STORAGE {
        if left <= 0 {
            break;
        }
        if let Some(Some(s)) = inv.get_mut(slot) {
            if similar(s, item) && s.count < max {
                let put = (max - s.count).min(left);
                s.count += put;
                left -= put;
                if !simulate {
                    host.set_slot(player, slot, Some(s.clone()));
                }
            }
        }
    }
    for slot in STORAGE {
        if left <= 0 {
            break;
        }
        if inv.get(slot).is_some_and(Option::is_none) {
            let put = max.min(left);
            let mut stack = item.clone();
            stack.count = put;
            left -= put;
            inv[slot] = Some(stack.clone());
            if !simulate {
                host.set_slot(player, slot, Some(stack));
            }
        }
    }
    left
}
fn held_item(host: &mut dyn Host, player: Uuid) -> Option<Item> {
    let slot = host.held_slot(player);
    host.inventory(player).get(slot).cloned().flatten()
}
fn set_held(host: &mut dyn Host, player: Uuid, item: Option<Item>) {
    let slot = host.held_slot(player);
    host.set_slot(player, slot, item);
}

/// Worth.getPrice: `worth.<name>.0` (a section with one key), `worth.<name>.*`, then `worth.<name>`.
pub fn price(ess: &Ess, item: &Item) -> Option<Dec> {
    let worth = ess.read_yaml("worth.yml");
    let entry = worth.get("worth").and_then(|w| w.get(worth_key(&item.id)))?;
    let positive = |v: Option<&Value>| v.and_then(Dec::from_value).filter(|d| d.signum() >= 0);
    let mut result = None;
    if let Some(section) = entry.as_object() {
        if section.len() == 1 {
            result = positive(section.get("0"));
        }
        if result.is_none() {
            result = positive(section.get("*"));
        }
    } else {
        result = positive(Some(entry));
    }
    result
}
fn set_price(ess: &Ess, item: &Item, price: f64) {
    let mut worth = ess.read_yaml("worth.yml");
    if !worth.is_object() {
        worth = json!({});
    }
    ess_yaml::set(&mut worth, &format!("worth.{}", worth_key(&item.id)), json!(price));
    ess.write_yaml("worth.yml", &worth);
}
/// Settings.getMultiplier: the best `sell-multipliers.<name>` the player has essentials.sell.multiplier.<name> for.
fn multiplier(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> Dec {
    let mut best = config_dec(ess, "sell-multipliers.default", "1");
    let Some(Value::Object(section)) = ess.config("sell-multipliers").cloned() else { return best };
    for (name, value) in section {
        if ess.has(host, Sender::Player(player), &format!("essentials.sell.multiplier.{name}")) {
            let v = Dec::from_value(&value).unwrap_or(Dec::ZERO);
            if v.cmp(&best) == Ordering::Greater {
                best = v;
            }
        }
    }
    best
}

/// Worth.getAmount: how many to sell (`64`, `-1` all but one, `2s` two stacks, nothing = all).
fn sell_amount(ess: &mut Ess, host: &mut dyn Host, player: Uuid, item: &Item, args: &[String], bulk: bool) -> Result<i32, CmdError> {
    if item.id == "minecraft:air" {
        return Err(CmdError::Tl("itemSellAir", vec![]));
    }
    let mut amount: i64 = 0;
    if args.len() > 1 {
        let digits: String = args[1].chars().filter(char::is_ascii_digit).collect();
        amount = digits.parse::<i32>().map_err(|_| CmdError::Usage(None))? as i64;
        if args[1].starts_with('-') {
            amount = -amount;
        }
    }
    let stack = args.len() > 1 && args[1].ends_with('s');
    let require_stack = ess.config_bool(&format!("trade-in-stacks.{}", worth_key(&item.id)), false);
    if require_stack && !stack {
        return Err(CmdError::Tl("itemMustBeStacked", vec![]));
    }
    let max = count_similar(host, player, item) as i64;
    let max_stack = host.max_stack(&item.id).max(1) as i64;
    if stack {
        amount *= max_stack;
    }
    if amount < 1 {
        amount += max;
    }
    if require_stack {
        amount -= amount % max_stack;
    }
    if (amount > max || amount < 1) && !bulk {
        ess.send_tl(host, Sender::Player(player), "itemNotEnough2", &[]);
        ess.send_tl(host, Sender::Player(player), "itemNotEnough3", &[]);
        return Err(CmdError::Tl("itemNotEnough1", vec![]));
    }
    Ok(amount.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

/// ItemDb.getMatching: hand, inventory/invent/all, blocks, or one item.
fn matching_items(ess: &Ess, host: &mut dyn Host, player: Uuid, args: &[String]) -> Result<Vec<Item>, CmdError> {
    let mut out = Vec::new();
    let first = args.first().map(|a| a.to_lowercase());
    match first.as_deref() {
        None | Some("hand") => out.extend(held_item(host, player)),
        Some("inventory" | "invent" | "all") => out.extend(host.inventory(player).into_iter().flatten().filter(|i| i.id != "minecraft:air")),
        Some("blocks") => {
            for i in host.inventory(player).into_iter().flatten() {
                if i.id != "minecraft:air" && host.item_is_block(&i.id) {
                    out.push(i);
                }
            }
        }
        Some(_) => out.push(resolve_item(ess, host, &args[0])?),
    }
    if out.is_empty() || out[0].id == "minecraft:air" {
        return Err(CmdError::Tl("itemSellAir", vec![]));
    }
    Ok(out)
}

// ===== Trades (Trade.java) ===================================================================================

/// What a sign or command costs or pays: money, items, experience, or a command's configured cost.
#[derive(Clone, Debug)]
enum Trade {
    Money(Dec),
    Item(Item),
    Exp(i64),
    /// `command-costs.<name>`, with a fallback trade when that cost is zero.
    Command(String, Option<Box<Trade>>),
}

#[derive(Clone, Copy, PartialEq)]
enum Overflow {
    Abort,
    Drop,
    Return,
}

impl Trade {
    fn money(&self) -> Option<&Dec> {
        if let Trade::Money(m) = self { Some(m) } else { None }
    }
    fn item(&self) -> Option<&Item> {
        if let Trade::Item(i) = self { Some(i) } else { None }
    }
    /// The kind for "same type" checks (TradeType).
    fn kind(&self) -> u8 {
        match self {
            Trade::Exp(_) => 1,
            Trade::Item(_) => 2,
            _ => 0,
        }
    }
    fn command_cost(&self, ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> Dec {
        let Trade::Command(command, fallback) = self else { return Dec::ZERO };
        if command.is_empty() {
            return Dec::ZERO;
        }
        let mut cost = command_cost(ess, command.trim_start_matches('/'));
        if cost.signum() == 0 {
            if let Some(f) = fallback {
                cost = f.command_cost(ess, host, player);
            }
        }
        if cost.signum() != 0 && (ess.has(host, Sender::Player(player), "essentials.nocommandcost.all") || ess.has(host, Sender::Player(player), &format!("essentials.nocommandcost.{command}"))) {
            return Dec::ZERO;
        }
        cost
    }
    /// Trade.isAffordableFor.
    fn affordable(&self, ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> Result<(), CmdError> {
        match self {
            Trade::Money(m) if m.signum() > 0 && !can_afford(ess, host, player, m) => Err(CmdError::Tl("notEnoughMoney", vec![cur(ess, m)])),
            Trade::Item(i) if !contains_at_least(host, player, i, i.count) => Err(CmdError::Tl("missingItems", vec![Arg::from(i.count), Arg::Plain(material(&i.id))])),
            Trade::Command(..) => {
                let cost = self.command_cost(ess, host, player);
                if cost.signum() > 0 && !can_afford(ess, host, player, &cost) {
                    return Err(CmdError::Tl("notEnoughMoney", vec![cur(ess, &cost)]));
                }
                Ok(())
            }
            Trade::Exp(e) if *e > 0 && host.exp(player) < *e => Err(CmdError::Tl("notEnoughExperience", vec![])),
            _ => Ok(()),
        }
    }
    /// Trade.pay: what didn't fit (Return), None otherwise.
    fn pay(&self, ess: &mut Ess, host: &mut dyn Host, player: Uuid, overflow: Overflow) -> Result<Option<Item>, CmdError> {
        match self {
            Trade::Money(m) if m.signum() > 0 => give_money(ess, host, player, m, None)?,
            Trade::Item(item) => {
                if overflow == Overflow::Abort && add_items(host, player, item, true) > 0 {
                    return Ok(Some(item.clone()));
                }
                let left = add_items(host, player, item, false);
                if left > 0 {
                    let mut rest = item.clone();
                    rest.count = left;
                    if overflow == Overflow::Return {
                        return Ok(Some(rest));
                    }
                    if let Some(at) = host.location(player) {
                        let max = host.max_stack(&item.id).max(1);
                        let mut spill = left;
                        while spill > 0 {
                            let mut drop = item.clone();
                            drop.count = spill.min(max);
                            spill -= drop.count;
                            host.drop_item(&at, &drop);
                        }
                    }
                }
            }
            Trade::Exp(e) => {
                let total = host.exp(player) + e;
                host.set_exp(player, total);
            }
            _ => {}
        }
        Ok(None)
    }
    /// Trade.charge.
    fn charge(&self, ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> Result<(), CmdError> {
        match self {
            Trade::Money(m) => {
                if !can_afford(ess, host, player, m) && m.signum() > 0 {
                    return Err(CmdError::Tl("notEnoughMoney", vec![cur(ess, m)]));
                }
                take_money(ess, host, player, m, None);
            }
            Trade::Item(i) => {
                if !contains_at_least(host, player, i, i.count) {
                    return Err(CmdError::Tl("missingItems", vec![Arg::from(i.count), Arg::Plain(material(&i.id).replace('_', " "))]));
                }
                remove_items(host, player, i, i.count);
            }
            Trade::Command(..) => {
                let cost = self.command_cost(ess, host, player);
                if !can_afford(ess, host, player, &cost) && cost.signum() > 0 {
                    return Err(CmdError::Tl("notEnoughMoney", vec![cur(ess, &cost)]));
                }
                take_money(ess, host, player, &cost, None);
            }
            Trade::Exp(e) => {
                let have = host.exp(player);
                if have < *e && *e > 0 {
                    return Err(CmdError::Tl("notEnoughExperience", vec![]));
                }
                host.set_exp(player, have - e);
            }
        }
        Ok(())
    }
}

// ===== Command costs ==========================================================================================

/// Settings.getCommandCost: `command-costs.<name>` (`.` and `/` read as `_`), zero when unset.
fn command_cost(ess: &Ess, name: &str) -> Dec {
    let key = name.replace(['.', '/'], "_").to_lowercase();
    let Some(Value::Object(costs)) = ess.config("command-costs") else { return Dec::ZERO };
    let Some((_, v)) = costs.iter().find(|(k, _)| k.to_lowercase() == key) else { return Dec::ZERO };
    match v {
        Value::Number(_) => Dec::from_value(v).unwrap_or(Dec::ZERO),
        Value::String(s) => {
            let symbol = currency_symbol(ess);
            let cleaned: String = s.trim().replace('$', "").replace(&symbol, "").chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '.').collect();
            Dec::parse(&cleaned).unwrap_or(Dec::ZERO)
        }
        _ => Dec::ZERO,
    }
}

/// Charges a player a `command-costs` entry (`chat-shout`, `chat-question`, `kit-tools`...) unless exempt by
/// essentials.nocommandcost.all / .<key>: Ok after charging (or when free), Err(notEnoughMoney) when they
/// can't afford it. For other areas (EssentialsChat's shout/question costs).
pub fn charge_command_cost(ess: &mut Ess, host: &mut dyn Host, player: Uuid, key: &str) -> Result<(), CmdError> {
    let trade = Trade::Command(key.to_string(), None);
    trade.affordable(ess, host, player)?;
    trade.charge(ess, host, player)
}

/// EssentialsCommand.run's charge for players: `command-costs.<command name>`. Without a post-command hook in
/// the core the charge happens before the command runs (see CORE-REQUESTS.md).
fn on_command_cost(ess: &mut Ess, host: &mut dyn Host, sender: Sender, line: &str) -> Allow {
    let Sender::Player(player) = sender else { return Allow::Yes };
    let label = line.trim_start_matches('/').split_whitespace().next().unwrap_or("");
    let Some(name) = ess.command_for(label) else { return Allow::Yes };
    if command_cost(ess, name).signum() == 0 || !ess.has(host, sender, &format!("essentials.{name}")) {
        return Allow::Yes;
    }
    match charge_command_cost(ess, host, player, name) {
        Ok(()) => Allow::Yes,
        Err(e) => {
            show_error(ess, host, sender, e);
            Allow::No
        }
    }
}

/// Essentials.showError: `Error: <message>`.
fn show_error(ess: &mut Ess, host: &mut dyn Host, to: Sender, e: CmdError) {
    match e {
        CmdError::Tl(key, args) => {
            let msg = ess.tl(key, &args);
            ess.send_tl(host, to, "errorWithMessage", &[Arg::Parsed(msg)]);
        }
        CmdError::Msg(m) => ess.send_tl(host, to, "errorWithMessage", &[Arg::Plain(m)]),
        CmdError::PlayerNotFound => {
            let msg = ess.tl("playerNotFound", &[]);
            ess.send_tl(host, to, "errorWithMessage", &[Arg::Parsed(msg)]);
        }
        CmdError::Usage(_) | CmdError::Quiet => {}
    }
}

// ===== /balance ==================================================================================================

fn shown_name(ctx: &mut Ctx, player: Uuid) -> String {
    if ctx.ess.is_vanished(player) { ctx.name_of(player) } else { display_name(ctx.ess, ctx.host, player) }
}

fn cmd_balance(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let others = ctx.is_player() && args.len() == 1 && ctx.has("essentials.balance.others");
    let target = match ctx.sender {
        Sender::Console => {
            let arg = args.first().ok_or(CmdError::Usage(None))?;
            Some(get_player(ctx, arg, true)?)
        }
        Sender::Player(_) if others => Some(get_player(ctx, &args[0], false)?),
        Sender::Player(_) if args.len() < 2 => None,
        Sender::Player(_) => return Err(CmdError::Usage(None)),
    };
    match target {
        Some(t) => {
            let name = shown_name(ctx, t);
            let bal = money(ctx.ess, t);
            let amount = cur(ctx.ess, &bal);
            ctx.tl("balanceOther", &[Arg::Plain(name), amount]);
        }
        None => {
            let me = ctx.player()?;
            let bal = money(ctx.ess, me);
            let amount = cur(ctx.ess, &bal);
            ctx.tl("balance", &[amount]);
        }
    }
    Ok(())
}

/// EssentialsCommand.getPlayer(server, arg, getHidden, getOffline=true).
fn get_player(ctx: &mut Ctx, search: &str, hidden: bool) -> Result<Uuid, CmdError> {
    let viewer = if hidden { Sender::Console } else { ctx.sender };
    ctx.ess.match_player(ctx.host, viewer, search, true).ok_or(CmdError::PlayerNotFound)
}

// ===== /balancetop ==================================================================================================

const BALTOP_CACHE_MS: i64 = 2 * 60 * 1000;

#[derive(Default)]
struct EcoState {
    baltop_lines: Vec<String>,
    baltop_age: i64,
    /// payer -> (payee -> amount) awaiting confirmation (User.getConfirmingPayments).
    confirming: HashMap<Uuid, HashMap<Uuid, Dec>>,
    /// Last sign use (User.lastThrottledAction).
    throttle: HashMap<Uuid, i64>,
}

fn cmd_balancetop(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let mut page = 0i64;
    let mut force = false;
    if let Some(a) = args.first() {
        match a.parse::<i64>() {
            Ok(n) => page = n,
            Err(_) => force = a.eq_ignore_ascii_case("force") && (!ctx.is_player() || ctx.has("essentials.balancetop.force")),
        }
    }
    let now = ctx.host.now_millis();
    let age = ctx.ess.state::<EcoState>().baltop_age;
    if !force && age > now - BALTOP_CACHE_MS {
        output_baltop(ctx, page);
        return Ok(());
    }
    let users = ctx.ess.users.all().len();
    if users > 50 {
        ctx.tl("orderBalances", &[Arg::from(users)]);
    }
    if eco_disabled(ctx.ess) {
        return Ok(());
    }
    calculate_baltop(ctx.ess, ctx.host);
    output_baltop(ctx, page);
    Ok(())
}

/// BalanceTopImpl.calculateBalanceTopMap and Commandbalancetop.Viewer: the cached lines.
fn calculate_baltop(ess: &mut Ess, host: &mut dyn Host) {
    let mut entries: Vec<(Uuid, String, Dec)> = Vec::new();
    let mut total = Dec::ZERO;
    let online = host.online();
    for u in ess.users.all() {
        if ess.users.get(u).is_none() && !ess.users.exists(u) {
            continue;
        }
        let npc = ess.users.get(u).is_some_and(|d| d.bool("npc"));
        if npc && !ess.config_bool("npcs-in-balance-ranking", false) {
            continue;
        }
        let exempt = if online.contains(&u) {
            let e = ess.has(host, Sender::Player(u), "essentials.balancetop.exclude");
            if ess.users.get(u).map(|d| d.bool("baltop-exempt")) != Some(e) {
                ess.users.user(u).set("baltop-exempt", json!(e));
            }
            e
        } else {
            ess.users.get(u).is_some_and(|d| d.bool("baltop-exempt"))
        };
        if exempt {
            continue;
        }
        let m = money(ess, u);
        total = total.add(&m);
        let name = if !online.contains(&u) {
            ess.users.get(u).and_then(|d| d.name()).unwrap_or_else(|| uuid_string(u))
        } else if ess.is_vanished(u) {
            user_name(ess, host, u)
        } else {
            display_name(ess, host, u)
        };
        entries.push((u, name, m));
    }
    entries.sort_by(|a, b| b.2.cmp(&a.2));
    let limit = ess.config_i64("baltop-entry-limit", -1);
    if limit >= 0 {
        entries.truncate(limit as usize);
    }
    let mut lines = vec![ess.tl("serverTotal", &[cur(ess, &total)])];
    let min_balance = config_dec(ess, "baltop-requirements.minimum-balance", "0");
    let min_playtime = ess.config_i64("baltop-requirements.minimum-playtime", 0);
    let show_zero = ess.config_bool("show-zero-baltop", true);
    for (pos, (u, name, balance)) in entries.iter().enumerate() {
        // Offline players' statistics aren't available here: their playtime check is skipped (playtime -1).
        let playtime = if online.contains(u) { host.statistic(*u, "minecraft:custom/minecraft:play_time") } else { -1 };
        let secs = (playtime / 20).max(0);
        if (show_zero || balance.signum() > 0) && balance.cmp(&min_balance) != Ordering::Less && (playtime == -1 || secs >= min_playtime) {
            lines.push(ess.tl("balanceTopLine", &[Arg::from(pos + 1), Arg::Plain(name.clone()), cur(ess, balance)]));
        }
    }
    let now = host.now_millis();
    let state = ess.state::<EcoState>();
    state.baltop_lines = lines;
    state.baltop_age = now;
}

fn output_baltop(ctx: &mut Ctx, page: i64) {
    let age = ctx.ess.state::<EcoState>().baltop_age;
    let date = chrono::DateTime::from_timestamp_millis(age)
        .map(|d| d.with_timezone(&chrono::Local).format("%-m/%-d/%y, %-I:%M %p").to_string())
        .unwrap_or_default();
    ctx.tl("balanceTop", &[Arg::Plain(date)]);
    let lines = ctx.ess.state::<EcoState>().baltop_lines.clone();
    show_page(ctx.ess, ctx.host, ctx.sender, &lines, page, "balancetop");
}

/// TextPager.showPage for text without chapters: 9 lines a page.
fn show_page(ess: &mut Ess, host: &mut dyn Host, to: Sender, lines: &[String], page: i64, command: &str) {
    let page = page.max(1);
    let end = lines.iter().position(|l| l.starts_with('#')).unwrap_or(lines.len()) as i64;
    let pages = end / 9 + i64::from(end % 9 > 0);
    if page > pages {
        ess.send_tl(host, to, "infoUnknownChapter", &[]);
        return;
    }
    let mut title = command.to_string();
    if let Some(first) = title.get(..1).map(str::to_uppercase) {
        title.replace_range(..1, &first);
    }
    ess.send_tl(host, to, "infoPages", &[Arg::from(page), Arg::from(pages), Arg::Plain(title)]);
    let start = (page - 1) * 9;
    for i in start..end.min(start + 9) {
        ess.send_mini(host, to, &format!("<reset>{}", lines[i as usize]));
    }
    if page < pages {
        ess.send_tl(host, to, "readNextPage", &[Arg::Plain(command.to_string()), Arg::from(page + 1)]);
    }
}

// ===== /eco ==========================================================================================================

/// EssentialsLoopCommand.loopOfflinePlayersConsumer (single name matches): the users a target means.
fn loop_offline(ctx: &mut Ctx, wildcards: bool, term: &str) -> Result<Vec<Uuid>, CmdError> {
    if term.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    if let (Sender::Player(p), "@s" | "@p") = (ctx.sender, term) {
        return Ok(vec![p]);
    }
    if let Some(u) = parse_uuid(term) {
        return if ctx.ess.users.exists(u) || is_online(ctx.host, u) { Ok(vec![u]) } else { Err(CmdError::PlayerNotFound) };
    }
    if wildcards && term == "**" {
        let mut all = ctx.ess.users.all();
        for p in ctx.host.online() {
            if !all.contains(&p) {
                all.push(p);
            }
        }
        return Ok(all);
    }
    if wildcards && term == "*" {
        let sender = ctx.sender;
        let skip_hidden = ctx.is_player() && !ctx.has("essentials.vanish.interact");
        let mut out = Vec::new();
        for p in ctx.host.online() {
            if skip_hidden && Sender::Player(p) != sender && ctx.ess.is_vanished(p) {
                continue;
            }
            out.push(p);
        }
        return Ok(out);
    }
    Ok(vec![get_player(ctx, term, true)?])
}

fn cmd_eco(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let cmd = args[0].to_uppercase();
    if !matches!(cmd.as_str(), "GIVE" | "TAKE" | "SET" | "RESET") {
        return Err(CmdError::Usage(None));
    }
    let amount = if cmd == "RESET" {
        starting_balance(ctx.ess)
    } else {
        let raw = args.get(2).ok_or(CmdError::Usage(None))?;
        parse_amount(raw).ok_or(CmdError::Usage(None))?
    };
    // Commandeco's percent form (`10%`) can't get past parseStringToBDecimal (an InvalidModifierException), as in
    // EssentialsX: it shows the usage.
    for target in loop_offline(ctx, true, &args[1].clone())? {
        let sender = ctx.sender;
        match cmd.as_str() {
            "GIVE" => give_money(ctx.ess, ctx.host, target, &amount, Some(sender))?,
            "TAKE" => {
                let min = min_money(ctx.ess);
                if money(ctx.ess, target).sub(&amount).cmp(&min) != Ordering::Less {
                    take_money(ctx.ess, ctx.host, target, &amount, Some(sender));
                } else {
                    let shown = cur(ctx.ess, &min);
                    show_error(ctx.ess, ctx.host, sender, CmdError::Tl("minimumBalanceError", vec![shown]));
                }
            }
            _ => {
                let (min, max) = (min_money(ctx.ess), max_money(ctx.ess));
                let value = if amount.cmp(&min) == Ordering::Less {
                    min
                } else if amount.cmp(&max) == Ordering::Greater {
                    max
                } else {
                    amount.clone()
                };
                set_money(ctx.ess, ctx.host, target, &value, true)?;
                let bal = money(ctx.ess, target);
                let shown = cur(ctx.ess, &bal);
                ctx.tl_to(Sender::Player(target), "setBal", std::slice::from_ref(&shown));
                let name = display_name(ctx.ess, ctx.host, target);
                ctx.tl("setBalOthers", &[Arg::Plain(name), shown]);
            }
        }
    }
    Ok(())
}

// ===== /pay, /paytoggle, /payconfirmtoggle ==============================================================================

fn prompting_pay_confirm(ess: &mut Ess, player: Uuid) -> bool {
    if let Some(v) = ess.users.get(player).and_then(|u| u.get("confirm-pay").and_then(Value::as_bool)) {
        return v;
    }
    ess.config_list("default-enabled-confirm-commands").iter().any(|c| c.eq_ignore_ascii_case("pay"))
}
fn accepting_pay(ess: &mut Ess, player: Uuid) -> bool {
    ess.users.get(player).and_then(|u| u.get("accepting-pay").and_then(Value::as_bool)).unwrap_or(true)
}
/// UserData.isIgnoredPlayer: `ignore` holds the uuid and the ignored player isn't essentials.chat.ignoreexempt.
fn ignores(ess: &mut Ess, host: &mut dyn Host, player: Uuid, other: Uuid) -> bool {
    let listed = ess.users.get(player).and_then(|u| u.get("ignore").cloned()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    listed.iter().filter_map(Value::as_str).filter_map(parse_uuid).any(|u| u == other) && !ess.has(host, Sender::Player(other), "essentials.chat.ignoreexempt")
}

fn cmd_pay(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let raw = &args[1];
    if raw.contains('-') {
        return Err(CmdError::Tl("payMustBePositive", vec![]));
    }
    if !raw.chars().any(|c| c.is_ascii_digit() || c == '.') {
        return Err(CmdError::Usage(None));
    }
    let amount = parse_amount(raw).ok_or(CmdError::Usage(None))?;
    let minimum = minimum_pay_amount(ctx.ess);
    if amount.cmp(&minimum) == Ordering::Less {
        return Err(CmdError::Tl("minimumPayAmount", vec![Arg::Parsed(display_currency_exactly(ctx.ess, &minimum))]));
    }
    let can_pay_offline = ctx.has("essentials.pay.offline");
    if !can_pay_offline && args[0] == "**" {
        ctx.tl("payOffline", &[]);
        return Ok(());
    }
    let multiple = ctx.has("essentials.pay.multiple");
    let mut inform = false;
    for target in loop_offline(ctx, multiple, &args[0].clone())? {
        let hidden = ctx.ess.is_vanished(target) && !ctx.has("essentials.vanish.see");
        if (!is_online(ctx.host, target) || hidden) && !can_pay_offline {
            ctx.tl("payOffline", &[]);
            continue;
        }
        if !accepting_pay(ctx.ess, target) || (ctx.ess.config_bool("pay-excludes-ignore-list", false) && ignores(ctx.ess, ctx.host, target, me)) {
            let name = display_name(ctx.ess, ctx.host, target);
            ctx.tl("notAcceptingPay", &[Arg::Plain(name)]);
            continue;
        }
        if prompting_pay_confirm(ctx.ess, me) {
            let confirmed = ctx.ess.state::<EcoState>().confirming.get(&me).and_then(|m| m.get(&target)) == Some(&amount);
            if !confirmed {
                let state = ctx.ess.state::<EcoState>();
                let map = state.confirming.entry(me).or_default();
                if !inform {
                    map.clear();
                    inform = true;
                }
                map.insert(target, amount.clone());
                continue;
            }
        }
        // User.payUser.
        if amount.signum() <= 0 {
            return Err(CmdError::Tl("payMustBePositive", vec![]));
        }
        if !can_afford(ctx.ess, ctx.host, me, &amount) {
            return Err(CmdError::Tl("notEnoughMoney", vec![cur(ctx.ess, &amount)]));
        }
        let mine = money(ctx.ess, me).sub(&amount);
        set_money(ctx.ess, ctx.host, me, &mine, true)?;
        let theirs = money(ctx.ess, target).add(&amount);
        if set_money(ctx.ess, ctx.host, target, &theirs, true).is_err() {
            ctx.tl("maxMoney", &[]);
            let back = money(ctx.ess, me).add(&amount);
            let _ = set_money(ctx.ess, ctx.host, me, &back, true);
            continue;
        }
        let (my_name, their_name) = (display_name(ctx.ess, ctx.host, me), display_name(ctx.ess, ctx.host, target));
        let shown = cur(ctx.ess, &amount);
        ctx.tl("moneySentTo", &[shown.clone(), Arg::Plain(their_name)]);
        ctx.tl_to(Sender::Player(target), "moneyRecievedFrom", &[shown, Arg::Plain(my_name)]);
        if let Some(m) = ctx.ess.state::<EcoState>().confirming.get_mut(&me) {
            m.remove(&target);
        }
        let (me_name, target_name) = (user_name(ctx.ess, ctx.host, me), user_name(ctx.ess, ctx.host, target));
        let at = player_block(ctx.host, me);
        let balance = money(ctx.ess, me);
        let t = Trade::Money(amount.clone());
        log_trade(ctx.ess, ctx.host, "Command", "Pay", "Player", Some(&me_name), Some(&t), Some(&target_name), Some(&t), at.as_ref().map(|(w, p)| (w.as_str(), *p)), Some(&balance));
    }
    if inform {
        let command = format!("/{} {}", ctx.label, args.join(" "));
        let shown = cur(ctx.ess, &amount);
        ctx.tl("confirmPayment", &[shown, Arg::Plain(command)]);
    }
    Ok(())
}

fn cmd_payconfirmtoggle(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let mut confirm = !prompting_pay_confirm(ctx.ess, me);
    if ctx.label.contains("payconfirmon") {
        confirm = true;
    } else if ctx.label.contains("payconfirmoff") {
        confirm = false;
    }
    ctx.ess.users.user(me).set("confirm-pay", json!(confirm));
    ctx.tl(if confirm { "payConfirmToggleOn" } else { "payConfirmToggleOff" }, &[]);
    ctx.ess.state::<EcoState>().confirming.remove(&me);
    Ok(())
}

/// EssentialsToggleCommand.matchToggleArgument.
fn toggle_arg(arg: &str) -> Option<bool> {
    let l = arg.to_lowercase();
    if l == "on" || arg.starts_with("ena") || arg == "1" {
        Some(true)
    } else if l == "off" || arg.starts_with("dis") || arg == "0" {
        Some(false)
    } else {
        None
    }
}

fn toggle_pay(ctx: &mut Ctx, target: Uuid, enabled: Option<bool>) {
    let enabled = enabled.unwrap_or_else(|| !accepting_pay(ctx.ess, target));
    ctx.ess.users.user(target).set("accepting-pay", json!(enabled));
    ctx.tl_to(Sender::Player(target), if enabled { "payToggleOn" } else { "payToggleOff" }, &[]);
    if ctx.sender != Sender::Player(target) {
        let name = display_name(ctx.ess, ctx.host, target);
        ctx.tl(if enabled { "payEnabledFor" } else { "payDisabledFor" }, &[Arg::Plain(name)]);
    }
}

/// EssentialsToggleCommand.toggleOtherPlayers.
fn toggle_others(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() || args[0].trim().len() < 2 {
        return Err(CmdError::PlayerNotFound);
    }
    let target = ctx.online_player(&args[0])?;
    let enabled = args.get(1).and_then(|a| toggle_arg(a));
    toggle_pay(ctx, target, enabled);
    Ok(())
}

fn cmd_paytoggle(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else { return toggle_others(ctx, args) };
    if ctx.label.contains("payon") {
        toggle_pay(ctx, me, Some(true));
    } else if ctx.label.contains("payoff") {
        toggle_pay(ctx, me, Some(false));
    } else if args.len() == 1 {
        let toggle = toggle_arg(&args[0]);
        if toggle.is_none() && ctx.has("essentials.paytoggle.others") {
            toggle_others(ctx, args)?;
        } else {
            toggle_pay(ctx, me, toggle);
        }
    } else if args.len() == 2 && ctx.has("essentials.paytoggle.others") {
        toggle_others(ctx, args)?;
    } else {
        toggle_pay(ctx, me, None);
    }
    Ok(())
}

// ===== /sell, /worth, /setworth ===================================================================================

fn cmd_sell(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let first = args[0].to_lowercase();
    if first == "hand" && !ctx.has("essentials.sell.hand") {
        return Err(CmdError::Tl("sellHandPermission", vec![]));
    }
    if matches!(first.as_str(), "inventory" | "invent" | "all") && !ctx.has("essentials.sell.bulk") {
        return Err(CmdError::Tl("sellBulkPermission", vec![]));
    }
    let mut stacks = matching_items(ctx.ess, ctx.host, me, args)?;
    let bulk = stacks.len() > 1;
    let mut total = Dec::ZERO;
    let mut count = 0;
    let mut not_sold: Vec<Item> = Vec::new();
    for i in 0..stacks.len() {
        let stack = stacks[i].clone();
        if !ctx.ess.config_bool("allow-selling-named-items", false) && has_custom_name(&stack) {
            if bulk {
                not_sold.push(stack);
                continue;
            }
            return Err(CmdError::Tl("cannotSellNamedItem", vec![]));
        }
        if stack.count <= 0 {
            continue;
        }
        match sell_item(ctx, me, &stack, args, bulk) {
            Ok(v) => {
                total = total.add(&v);
                count += 1;
                for other in stacks.iter_mut() {
                    if similar(other, &stack) {
                        other.count = 0;
                    }
                }
            }
            Err(e) if !bulk => return Err(e),
            Err(_) => {}
        }
    }
    if !not_sold.is_empty() {
        let names: Vec<String> = not_sold.iter().map(custom_name).collect();
        let sender = ctx.sender;
        show_error(ctx.ess, ctx.host, sender, CmdError::Tl("cannotSellTheseNamedItems", vec![Arg::Plain(names.join("§r, "))]));
    }
    if count != 1 {
        let shown = cur(ctx.ess, &total);
        ctx.tl(if first == "blocks" { "totalWorthBlocks" } else { "totalWorthAll" }, &[shown.clone(), shown]);
    }
    Ok(())
}

/// Commandsell.sellItem.
fn sell_item(ctx: &mut Ctx, me: Uuid, item: &Item, args: &[String], bulk: bool) -> Result<Dec, CmdError> {
    let amount = sell_amount(ctx.ess, ctx.host, me, item, args, bulk)?;
    let worth = price(ctx.ess, item).map(|p| {
        let m = multiplier(ctx.ess, ctx.host, me);
        p.mul(&m)
    });
    let Some(worth) = worth else { return Err(CmdError::Tl("itemCannotBeSold", vec![])) };
    let type_name = material(&item.id);
    if amount <= 0 {
        if !bulk {
            let zero = cur(ctx.ess, &Dec::ZERO);
            ctx.tl("itemSold", &[zero, Arg::from(0), Arg::Plain(type_name), Arg::Parsed(display_currency(ctx.ess, &worth))]);
        }
        return Ok(Dec::ZERO);
    }
    let result = worth.mul(&Dec::from_i64(amount as i64));
    let mut sold = item.clone();
    sold.count = amount;
    if !contains_at_least(ctx.host, me, &sold, amount) {
        return Err(CmdError::Msg("Trying to remove more items than are available.".into()));
    }
    remove_items(ctx.host, me, &sold, amount);
    let name = user_name(ctx.ess, ctx.host, me);
    let at = player_block(ctx.host, me);
    let balance = money(ctx.ess, me);
    log_trade(ctx.ess, ctx.host, "Command", "Sell", "Item", Some(&name), Some(&Trade::Item(sold.clone())), Some(&name), Some(&Trade::Money(result.clone())), at.as_ref().map(|(w, p)| (w.as_str(), *p)), Some(&balance));
    give_money(ctx.ess, ctx.host, me, &result, None)?;
    let shown_result = cur(ctx.ess, &result);
    let shown_worth = cur(ctx.ess, &worth);
    ctx.tl("itemSold", &[shown_result.clone(), Arg::from(amount), Arg::Plain(type_name.clone()), shown_worth.clone()]);
    let display = display_name(ctx.ess, ctx.host, me);
    let console = ctx.ess.tl("itemSoldConsole", &[Arg::Plain(name), Arg::Plain(type_name), shown_result, Arg::from(amount), shown_worth, Arg::Plain(display)]);
    ctx.ess.send_mini(ctx.host, Sender::Console, &console);
    Ok(result)
}

fn cmd_worth(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else {
        let name = args.first().ok_or(CmdError::Usage(None))?;
        let item = resolve_item(ctx.ess, ctx.host, name)?;
        item_worth(ctx, None, &item, args)?;
        return Ok(());
    };
    let mut stacks = matching_items(ctx.ess, ctx.host, me, args)?;
    let bulk = stacks.len() > 1;
    let mut total = Dec::ZERO;
    let mut count = 0;
    for i in 0..stacks.len() {
        let stack = stacks[i].clone();
        if stack.count <= 0 {
            continue;
        }
        match item_worth(ctx, Some(me), &stack, args) {
            Ok(v) => {
                total = total.add(&v);
                count += 1;
                for other in stacks.iter_mut() {
                    if similar(other, &stack) {
                        other.count = 0;
                    }
                }
            }
            Err(e) if !bulk => return Err(e),
            Err(_) => {}
        }
    }
    if count > 1 {
        let shown = cur(ctx.ess, &total);
        let blocks = args.first().is_some_and(|a| a.eq_ignore_ascii_case("blocks"));
        ctx.tl(if blocks { "totalSellableBlocks" } else { "totalSellableAll" }, &[shown.clone(), shown]);
    }
    Ok(())
}

/// Commandworth.itemWorth.
fn item_worth(ctx: &mut Ctx, user: Option<Uuid>, item: &Item, args: &[String]) -> Result<Dec, CmdError> {
    let mut amount = match user {
        None if args.len() > 1 => {
            let digits: String = args[1].chars().filter(char::is_ascii_digit).collect();
            digits.parse::<i32>().map_err(|_| CmdError::Usage(None))?
        }
        None => 1,
        Some(me) => sell_amount(ctx.ess, ctx.host, me, item, args, true)?,
    };
    let worth = price(ctx.ess, item).ok_or(CmdError::Tl("itemCannotBeSold", vec![]))?;
    if amount < 0 {
        amount = 0;
    }
    let result = worth.mul(&Dec::from_i64(amount as i64));
    let type_name = worth_key(&item.id);
    let (shown_result, shown_worth) = (cur(ctx.ess, &result), cur(ctx.ess, &worth));
    let dmg = damage(item);
    if dmg != 0 {
        ctx.tl("worthMeta", &[Arg::Plain(type_name), Arg::from(dmg), shown_result, Arg::from(amount), shown_worth]);
    } else {
        ctx.tl("worth", &[Arg::Plain(type_name), shown_result, Arg::from(amount), shown_worth]);
    }
    Ok(result)
}

fn cmd_setworth(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let (item, price_arg) = match ctx.sender {
        Sender::Player(me) if args.len() == 1 => (held_item(ctx.host, me).unwrap_or_else(|| Item::new("air", 1)), &args[0]),
        _ if args.len() >= 2 => (resolve_item(ctx.ess, ctx.host, &args[0])?, &args[1]),
        _ => return Err(CmdError::Usage(None)),
    };
    let value: f64 = price_arg.trim().parse().map_err(|_| CmdError::Msg(format!("For input string: \"{price_arg}\"")))?;
    set_price(ctx.ess, &item, value);
    ctx.tl("worthSet", &[]);
    Ok(())
}

// ===== Signs ========================================================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Anvil,
    Balance,
    Buy,
    Cartography,
    Disposal,
    Enchant,
    Free,
    GameMode,
    Grindstone,
    Heal,
    Info,
    Kit,
    Loom,
    Mail,
    Protection,
    Repair,
    Sell,
    Smithing,
    Spawnmob,
    Time,
    Trade,
    Warp,
    Weather,
    Workbench,
    RandomTeleport,
}

/// The Signs enum, in its order.
const KINDS: [Kind; 25] = [
    Kind::Anvil,
    Kind::Balance,
    Kind::Buy,
    Kind::Cartography,
    Kind::Disposal,
    Kind::Enchant,
    Kind::Free,
    Kind::GameMode,
    Kind::Grindstone,
    Kind::Heal,
    Kind::Info,
    Kind::Kit,
    Kind::Loom,
    Kind::Mail,
    Kind::Protection,
    Kind::Repair,
    Kind::Sell,
    Kind::Smithing,
    Kind::Spawnmob,
    Kind::Time,
    Kind::Trade,
    Kind::Warp,
    Kind::Weather,
    Kind::Workbench,
    Kind::RandomTeleport,
];

impl Kind {
    /// EssentialsSign.signName.
    fn name(self) -> &'static str {
        match self {
            Kind::Anvil => "Anvil",
            Kind::Balance => "Balance",
            Kind::Buy => "Buy",
            Kind::Cartography => "Cartography",
            Kind::Disposal => "Disposal",
            Kind::Enchant => "Enchant",
            Kind::Free => "Free",
            Kind::GameMode => "GameMode",
            Kind::Grindstone => "Grindstone",
            Kind::Heal => "Heal",
            Kind::Info => "Info",
            Kind::Kit => "Kit",
            Kind::Loom => "Loom",
            Kind::Mail => "Mail",
            Kind::Protection => "Protection",
            Kind::Repair => "Repair",
            Kind::Sell => "Sell",
            Kind::Smithing => "Smithing",
            Kind::Spawnmob => "Spawnmob",
            Kind::Time => "Time",
            Kind::Trade => "Trade",
            Kind::Warp => "Warp",
            Kind::Weather => "Weather",
            Kind::Workbench => "Workbench",
            Kind::RandomTeleport => "RandomTeleport",
        }
    }
    fn lower(self) -> String {
        self.name().to_lowercase()
    }
    fn by_key(key: &str) -> Option<Kind> {
        KINDS.into_iter().find(|k| k.name().eq_ignore_ascii_case(key))
    }
    /// areHeavyEventRequired / getBlocks: only [Protection] guards blocks.
    fn guards(self, mat: &str) -> bool {
        self == Kind::Protection && is_protected_block(mat)
    }
}

/// Settings._getEnabledSigns: the enabled signs, and whether sign handling runs at all (any sign or colour).
fn enabled_signs(ess: &Ess) -> (Vec<Kind>, bool) {
    let mut kinds = Vec::new();
    let mut on = false;
    for name in ess.config_list("enabledSigns") {
        let name = name.trim().to_uppercase();
        if name.is_empty() {
            continue;
        }
        if name == "COLOR" || name == "COLOUR" {
            on = true;
            continue;
        }
        if let Some(k) = Kind::by_key(&name) {
            kinds.push(k);
            on = true;
        }
    }
    (kinds, on)
}
fn unprotected_signs(ess: &Ess) -> Vec<Kind> {
    ess.config_list("unprotected-sign-names").iter().filter_map(|n| Kind::by_key(n.trim())).collect()
}
fn success_name(ess: &Ess, kind: Kind) -> String {
    mini_to_legacy(ess, &ess.tl("signFormatSuccess", &[Arg::Plain(kind.name().into())]))
}
fn template_name(ess: &Ess, kind: Kind) -> String {
    mini_to_legacy(ess, &ess.tl("signFormatTemplate", &[Arg::Plain(kind.name().into())]))
}
fn fail_name(ess: &Ess, kind: Kind) -> String {
    mini_to_legacy(ess, &ess.tl("signFormatFail", &[Arg::Plain(kind.name().into())]))
}
/// ChatColor.stripColor: `§` codes only.
fn strip_color(s: &str) -> String {
    ess_text::strip_legacy(s, false)
}
/// BlockSign.getLine: private-use glyphs removed.
fn clean_line(s: &str) -> String {
    s.chars().filter(|c| !('\u{f700}'..='\u{f747}').contains(c)).collect()
}

fn block_material(state: &str) -> String {
    let id = state.split('[').next().unwrap_or(state);
    material(id)
}
fn block_prop(state: &str, key: &str) -> Option<String> {
    let props = state.split_once('[')?.1.trim_end_matches(']');
    props.split(',').find_map(|kv| kv.split_once('=').filter(|(k, _)| k.trim() == key).map(|(_, v)| v.trim().to_string()))
}
fn is_sign_block(mat: &str) -> bool {
    mat.ends_with("_sign")
}
fn is_sign_post(mat: &str) -> bool {
    mat.ends_with("_sign") && !mat.contains("wall") && !mat.contains("hanging")
}
fn is_wall_sign(mat: &str) -> bool {
    mat.ends_with("_wall_sign") || mat.ends_with("_wall_hanging_sign")
}
fn is_protected_block(mat: &str) -> bool {
    matches!(mat, "chest" | "furnace" | "dispenser")
}
fn rel(pos: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]]
}
const NORTH: [i32; 3] = [0, 0, -1];
const SOUTH: [i32; 3] = [0, 0, 1];
const EAST: [i32; 3] = [1, 0, 0];
const WEST: [i32; 3] = [-1, 0, 0];
const UP: [i32; 3] = [0, 1, 0];
const DOWN: [i32; 3] = [0, -1, 0];

fn sign_at(host: &mut dyn Host, world: &str, pos: [i32; 3]) -> Option<[String; 4]> {
    let state = host.block(world, pos)?;
    if !is_sign_block(&block_material(&state)) {
        return None;
    }
    host.sign_lines(world, pos).map(|l| l.map(|s| clean_line(&s)))
}
/// EssentialsSign.isValidSign (the deprecated form: any `§1[...]` first line).
fn valid_sign_at(host: &mut dyn Host, world: &str, pos: [i32; 3]) -> bool {
    sign_at(host, world, pos).is_some_and(|l| l[0].starts_with("§1[") && l[0].ends_with(']'))
}
/// EssentialsSign.checkIfBlockBreaksSigns: a valid sign stands on the block or hangs on one of its sides.
fn block_holds_signs(host: &mut dyn Host, world: &str, pos: [i32; 3]) -> bool {
    let up = rel(pos, UP);
    if host.block(world, up).is_some_and(|s| is_sign_post(&block_material(&s))) && valid_sign_at(host, world, up) {
        return true;
    }
    for (face, name) in [(NORTH, "north"), (EAST, "east"), (SOUTH, "south"), (WEST, "west")] {
        let at = rel(pos, face);
        let Some(state) = host.block(world, at) else { continue };
        if is_wall_sign(&block_material(&state)) && block_prop(&state, "facing").as_deref() == Some(name) && valid_sign_at(host, world, at) {
            return true;
        }
    }
    false
}

/// FormatUtil.formatString(user, "essentials.signs", line): `&` codes the player may use become `§` codes,
/// others are stripped.
fn format_sign_line(ess: &mut Ess, host: &mut dyn Host, player: Uuid, line: &str) -> String {
    const NAMES: [(char, &str); 22] = [
        ('0', "black"),
        ('1', "dark_blue"),
        ('2', "dark_green"),
        ('3', "dark_aqua"),
        ('4', "dark_red"),
        ('5', "dark_purple"),
        ('6', "gold"),
        ('7', "gray"),
        ('8', "dark_gray"),
        ('9', "blue"),
        ('a', "green"),
        ('b', "aqua"),
        ('c', "red"),
        ('d', "light_purple"),
        ('e', "yellow"),
        ('f', "white"),
        ('k', "obfuscated"),
        ('l', "bold"),
        ('m', "strikethrough"),
        ('n', "underline"),
        ('o', "italic"),
        ('r', "reset"),
    ];
    let who = Sender::Player(player);
    let color = ess.has(host, who, "essentials.signs.color");
    let format = ess.has(host, who, "essentials.signs.format");
    let magic = ess.has(host, who, "essentials.signs.magic");
    let name = host.name(player).unwrap_or_default();
    let mut supported: Vec<char> = Vec::new();
    for (code, node) in NAMES {
        let mut ok = match code {
            '0'..='9' | 'a'..='f' => color,
            'k' => magic,
            _ => format,
        };
        // A node set for one code (isPermissionSet) adds or removes it.
        if let Some(set) = ess.perms.configured(player, &name, &format!("essentials.signs.{node}")) {
            ok = set;
        }
        if ok {
            supported.push(code);
        }
    }
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == '§' || c == '&') && i + 1 < chars.len() {
            let code = chars[i + 1].to_ascii_lowercase();
            if "0123456789abcdefklmnor".contains(code) {
                if supported.contains(&code) {
                    out.push('§');
                    out.push(code);
                }
                i += 2;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// One sign being handled: its lines (the edit's, or the block's) and where it is.
struct SignCx<'a> {
    ess: &'a mut Ess,
    host: &'a mut dyn Host,
    player: Uuid,
    /// EssentialsSign.getUsername: the name cut to 13 characters.
    username: String,
    world: String,
    pos: [i32; 3],
    lines: [String; 4],
    kind: Kind,
}

impl SignCx<'_> {
    fn line(&self, i: usize) -> String {
        self.lines[i].clone()
    }
    fn set(&mut self, i: usize, text: impl Into<String>) {
        self.lines[i] = text.into();
    }
    fn me(&self) -> Sender {
        Sender::Player(self.player)
    }
    fn has(&mut self, node: &str) -> bool {
        self.ess.has(self.host, Sender::Player(self.player), node)
    }
    fn tl(&mut self, key: &str, args: &[Arg]) {
        let to = self.me();
        self.ess.send_tl(self.host, to, key, args);
    }
    fn log(&mut self, subtype: &str, event: &str, charge: Option<&Trade>, pay: Option<&Trade>) {
        let name = self.username.clone();
        let balance = money(self.ess, self.player);
        let world = self.world.clone();
        log_trade(self.ess, self.host, "Sign", subtype, event, Some(&name), charge, Some(&name), pay, Some((&world, self.pos)), Some(&balance));
    }
    fn in_group(&mut self, group: &str) -> bool {
        let name = self.host.name(self.player).unwrap_or_default();
        self.ess.perms.groups_of(self.player, &name).iter().any(|g| g.eq_ignore_ascii_case(group))
    }

    // ----- EssentialsSign's trade helpers --------------------------------------------------------------
    fn integer(line: &str) -> Result<i64, CmdError> {
        line.trim().parse::<i32>().map(i64::from).map_err(|_| CmdError::Tl("invalidSign", vec![]))
    }
    fn integer_positive(line: &str) -> Result<i64, CmdError> {
        let q = Self::integer(line)?;
        if q < 1 {
            return Err(CmdError::Tl("moreThanZero", vec![]));
        }
        Ok(q)
    }
    /// getBigDecimal: the line without the currency symbol, as a BigDecimal.
    fn big_decimal(&self, line: &str) -> Result<Dec, CmdError> {
        let symbol = currency_symbol(self.ess);
        let text = line.replace(&symbol, "");
        Dec::parse(&text).ok_or_else(|| CmdError::Msg(format!("Character array is missing \"exponent\" mark 'e' or 'E'. ({text})")))
    }
    fn big_decimal_positive(&self, line: &str) -> Result<Dec, CmdError> {
        let q = self.big_decimal(line)?;
        if q.cmp(&Dec::parse("0.01").expect("decimal")) == Ordering::Less {
            return Err(CmdError::Tl("moreThanZero", vec![]));
        }
        Ok(q)
    }
    /// getMoney: `^[^0-9-.]?[.0-9]+[^0-9-.]?$` lines are money.
    fn money_line(&self, line: &str) -> Result<Option<Dec>, CmdError> {
        let chars: Vec<char> = line.chars().collect();
        let excluded = |c: char| c.is_ascii_digit() || c == '-' || c == '.';
        let body = |c: char| c.is_ascii_digit() || c == '.';
        let mut a = 0;
        let mut b = chars.len();
        if a < b && !body(chars[a]) {
            if excluded(chars[a]) {
                return Ok(None);
            }
            a += 1;
        }
        if b > a && !body(chars[b - 1]) {
            if excluded(chars[b - 1]) {
                return Ok(None);
            }
            b -= 1;
        }
        if a >= b || !chars[a..b].iter().all(|c| body(*c)) {
            return Ok(None);
        }
        self.big_decimal_positive(line).map(Some)
    }
    fn item_stack(&mut self, name: &str, quantity: i64) -> Result<Item, CmdError> {
        let mut item = resolve_item(self.ess, self.host, name)?;
        item.count = quantity as i32;
        Ok(item)
    }
    /// getTrade(sign, index, decrement): money, `<n> exp`, `<n> times`, `<n> <item>`, or the sign's command cost.
    fn trade(&mut self, index: usize, decrement: i64) -> Result<Trade, CmdError> {
        let line = self.line(index).trim().to_string();
        if line.is_empty() {
            return Ok(Trade::Command(format!("{}sign", self.kind.lower()), None));
        }
        if let Some(m) = self.money_line(&line)? {
            return Ok(Trade::Money(m));
        }
        let split = split_sign(&line, Some(2));
        if split.len() != 2 {
            return Err(CmdError::Tl("invalidCharge", vec![]));
        }
        let quantity = Self::integer_positive(&split[0])?;
        let item = split[1].to_lowercase();
        if item == "times" {
            self.set(index, format!("{} times", quantity - decrement));
            Ok(Trade::Command(format!("{}sign", self.kind.lower()), None))
        } else if item == "exp" || item == "xp" {
            self.set(index, format!("{quantity} exp"));
            Ok(Trade::Exp(quantity))
        } else {
            let stack = self.item_stack(&item, quantity)?;
            self.set(index, format!("{quantity} {item}"));
            Ok(Trade::Item(stack))
        }
    }
    /// validateTrade(sign, index): money lines are rewritten as short currency.
    fn validate_trade(&mut self, index: usize) -> Result<(), CmdError> {
        if self.line(index).trim().is_empty() {
            return Ok(());
        }
        if let Trade::Money(m) = self.trade(index, 0)? {
            let text = short_currency(self.ess, &m);
            self.set(index, text);
        }
        Ok(())
    }
    /// getTrade(sign, amountIndex, itemIndex): an amount line and an item (or exp) line.
    fn item_trade(&mut self, amount_index: usize, item_index: usize) -> Result<Trade, CmdError> {
        let item_type = self.line(item_index).trim().to_string();
        if item_type.eq_ignore_ascii_case("exp") || item_type.eq_ignore_ascii_case("xp") {
            return Ok(Trade::Exp(Self::integer_positive(&self.line(amount_index))?));
        }
        let mut item = self.item_stack(&item_type, 1)?;
        let max = self.host.max_stack(&item.id) as i64 * 41;
        let amount = Self::integer_positive(&self.line(amount_index))?.min(max);
        if item.id == "minecraft:air" || amount < 1 {
            return Err(CmdError::Tl("moreThanZero", vec![]));
        }
        item.count = amount as i32;
        Ok(Trade::Item(item))
    }
    fn validate_item_trade(&mut self, amount_index: usize, item_index: usize) -> Result<(), CmdError> {
        let item_type = self.line(item_index).trim().to_string();
        if item_type.eq_ignore_ascii_case("exp") || item_type.eq_ignore_ascii_case("xp") {
            let amount = Self::integer_positive(&self.line(amount_index))?;
            self.set(amount_index, amount.to_string());
            self.set(item_index, "exp");
            return Ok(());
        }
        let trade = self.item_trade(amount_index, item_index)?;
        let count = trade.item().map_or(0, |i| i.count);
        self.set(amount_index, count.to_string());
        self.set(item_index, item_type);
        Ok(())
    }
}

/// `[ :]+` splitting as Java's String.split (trailing empty strings dropped; `limit` keeps the rest whole).
fn split_sign(line: &str, limit: Option<usize>) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if (c == ' ' || c == ':') && limit.is_none_or(|l| out.len() + 1 < l) {
            out.push(std::mem::take(&mut cur));
            while chars.peek().is_some_and(|n| *n == ' ' || *n == ':') {
                chars.next();
            }
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    if limit.is_none() {
        while out.last().is_some_and(String::is_empty) && out.len() > 1 {
            out.pop();
        }
    }
    out
}


// ----- The signs' own behaviour (signs/Sign*.java) ---------------------------------------------------------

fn sign_err(key: &'static str) -> CmdError {
    CmdError::Tl(key, vec![])
}
/// `new SignException(ex, "errorWithMessage", ex.getMessage())`: the message wrapped once more.
fn wrapped(ess: &Ess, e: CmdError) -> CmdError {
    let msg = match e {
        CmdError::Tl(key, args) => ess.tl(key, &args),
        CmdError::Msg(m) => ess_text::escape_tags(&m),
        CmdError::PlayerNotFound => ess.tl("playerNotFound", &[]),
        CmdError::Usage(_) | CmdError::Quiet => String::new(),
    };
    CmdError::Tl("errorWithMessage", vec![Arg::Parsed(msg)])
}
fn find_enchantment(host: &mut dyn Host, name: &str) -> Option<(String, i32)> {
    let n = name.trim().to_lowercase();
    let n = n.trim_start_matches("minecraft:");
    host.enchantments().into_iter().find(|(id, _)| {
        let short = material(id);
        short == n || short.replace('_', "") == n.replace('_', "")
    })
}
fn enchant_level(item: &Item, id: &str) -> Option<i64> {
    item.components.get("enchantments").and_then(|e| e.get(id)).and_then(Value::as_i64)
}
fn kit_exists(ess: &Ess, name: &str) -> bool {
    let kits = ess.read_yaml("kits.yml");
    let has = |v: &Value| v.get("kits").and_then(Value::as_object).is_some_and(|k| k.keys().any(|key| key.eq_ignore_ascii_case(name)));
    has(&kits) || has(&ess.config)
}
fn warp_exists(ess: &Ess, name: &str) -> bool {
    let file: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    ess.dir.join("warps").join(format!("{file}.yml")).exists()
}
fn station_of(kind: Kind) -> Option<Station> {
    match kind {
        Kind::Anvil => Some(Station::Anvil),
        Kind::Cartography => Some(Station::Cartography),
        Kind::Grindstone => Some(Station::Grindstone),
        Kind::Loom => Some(Station::Loom),
        Kind::Smithing => Some(Station::Smithing),
        Kind::Workbench => Some(Station::Crafting),
        _ => None,
    }
}
fn container_title(text: &str) -> String {
    ess_text::legacy_to_mini(&ess_text::escape_tags(text), false)
}

/// onSignCreate of each sign: Ok(false) refuses the edit, Err marks the sign as failed.
fn sign_create(cx: &mut SignCx) -> Result<bool, CmdError> {
    match cx.kind {
        Kind::Balance | Kind::Mail | Kind::RandomTeleport | Kind::Workbench => Ok(true),
        // ContainerProvider is always there: the stations open through the Host.
        Kind::Anvil | Kind::Cartography | Kind::Grindstone | Kind::Loom | Kind::Smithing => Ok(true),
        Kind::Buy | Kind::Sell => {
            cx.validate_item_trade(1, 2)?;
            cx.validate_trade(3)?;
            Ok(true)
        }
        Kind::Free => {
            if let Err(e) = cx.item_stack(&cx.line(1), 1) {
                cx.set(1, "§c<item>");
                return Err(wrapped(cx.ess, e));
            }
            Ok(true)
        }
        Kind::Disposal => {
            if !cx.has("essentials.signs.disposal.name") {
                for i in 1..4 {
                    cx.set(i, "");
                }
            }
            Ok(true)
        }
        Kind::Enchant => {
            let item_name = cx.line(1);
            let any = item_name == "*" || item_name.eq_ignore_ascii_case("any");
            if !any {
                if let Err(e) = cx.item_stack(&item_name, 1) {
                    cx.set(1, "§c<item|any>");
                    return Err(e);
                }
            }
            let line = cx.line(2);
            let parts: Vec<&str> = line.split(':').collect();
            let Some((_, max)) = find_enchantment(cx.host, parts[0]) else {
                cx.set(2, "§c<enchant>");
                return Err(sign_err("enchantmentNotFound"));
            };
            let mut level: i64 = 1;
            if parts.len() > 1 {
                match parts[1].trim().parse::<i32>() {
                    Ok(l) => level = l as i64,
                    Err(_) => {
                        cx.set(2, "§c<enchant>");
                        return Err(wrapped(cx.ess, CmdError::Msg(format!("For input string: \"{}\"", parts[1]))));
                    }
                }
            }
            let allow_unsafe = cx.ess.config_bool("allow-unsafe-enchantments", false) && cx.has("essentials.enchantments.allowunsafe") && cx.has("essentials.signs.enchant.allowunsafe");
            if level < 0 || (!allow_unsafe && level > max as i64) {
                level = max as i64;
                cx.set(2, format!("{}:{level}", parts[0]));
            }
            cx.trade(3, 1)?;
            Ok(true)
        }
        Kind::GameMode => {
            if cx.line(1).is_empty() {
                cx.set(1, "Survival");
            }
            cx.validate_trade(2)?;
            Ok(true)
        }
        Kind::Heal => cx.validate_trade(1).map(|_| true),
        Kind::Info => cx.validate_trade(3).map(|_| true),
        Kind::Kit => {
            cx.validate_trade(3)?;
            let kit = cx.line(1).to_lowercase().trim().to_string();
            if kit.is_empty() {
                cx.set(1, "§dKit name!");
                return Ok(false);
            }
            if !kit_exists(cx.ess, &kit) {
                return Err(wrapped(cx.ess, sign_err("kitNotFound")));
            }
            let group = cx.line(2);
            if group.eq_ignore_ascii_case("Everyone") || group.eq_ignore_ascii_case("Everybody") {
                cx.set(2, "§2Everyone");
            }
            Ok(true)
        }
        Kind::Protection => {
            let me = cx.username.clone();
            cx.set(3, format!("§4{me}"));
            if has_adjacent_protected(cx.host, &cx.world, cx.pos) {
                let state = is_block_protected(cx.ess, cx.host, &cx.world, cx.pos, Some(cx.player), true);
                if state == Prot::NoSign || state == Prot::Owner || cx.has("essentials.signs.protection.override") {
                    cx.set(3, format!("§1{me}"));
                    return Ok(true);
                }
            }
            cx.tl("signProtectInvalidLocation", &[]);
            Ok(false)
        }
        Kind::Repair => {
            let target = cx.line(1);
            if target.is_empty() {
                cx.set(1, "Hand");
            } else if !target.eq_ignore_ascii_case("all") && !target.eq_ignore_ascii_case("hand") {
                cx.set(1, "§c<hand|all>");
                return Err(CmdError::Tl("invalidSignLine", vec![Arg::from(2)]));
            }
            cx.validate_trade(2)?;
            Ok(true)
        }
        Kind::Spawnmob => {
            let line = cx.line(1);
            if line.trim().is_empty() {
                return Err(CmdError::Tl("emptySignLine", vec![Arg::from(2)]));
            }
            let q = SignCx::integer_positive(&line)?;
            cx.set(1, q.to_string());
            cx.validate_trade(3)?;
            Ok(true)
        }
        Kind::Time | Kind::Weather => {
            if cx.line(1).is_empty() && cx.line(2).is_empty() && cx.line(3).is_empty() {
                return Ok(true);
            }
            cx.validate_trade(2)?;
            let value = cx.line(1);
            let (a, b, err) = if cx.kind == Kind::Time { ("Day", "Night", "onlyDayNight") } else { ("Sun", "Storm", "onlySunStorm") };
            for word in [a, b] {
                if value.eq_ignore_ascii_case(word) {
                    cx.set(1, format!("§2{word}"));
                    return Ok(true);
                }
            }
            if cx.kind == Kind::Weather {
                cx.set(1, "§c<sun|storm>");
            }
            Err(sign_err(err))
        }
        Kind::Trade => {
            ts_validate(cx, 1, false)?;
            ts_validate(cx, 2, true)?;
            let trade = ts_get(cx, 2, Amt::Rounded, true)?;
            let charge = ts_get(cx, 1, Amt::Rounded, false)?;
            if trade.kind() == charge.kind() && (trade.kind() != 2 || similar(trade.item().expect("item"), charge.item().expect("item"))) {
                return Err(sign_err("tradeSignSameType"));
            }
            trade.affordable(cx.ess, cx.host, cx.player)?;
            let me = cx.username.clone();
            cx.set(3, format!("§8{me}"));
            trade.charge(cx.ess, cx.host, cx.player)?;
            cx.log("Trade", "Create", Some(&trade), None);
            Ok(true)
        }
        Kind::Warp => {
            cx.validate_trade(3)?;
            let warp = cx.line(1);
            if warp.is_empty() {
                cx.set(1, "§c<Warp name>");
                return Err(CmdError::Tl("invalidSignLine", vec![Arg::from(1)]));
            }
            if !warp_exists(cx.ess, &warp) {
                return Err(sign_err("warpNotExist"));
            }
            let group = cx.line(2);
            if group.eq_ignore_ascii_case("Everyone") || group.eq_ignore_ascii_case("Everybody") {
                cx.set(2, "§2Everyone");
            }
            Ok(true)
        }
    }
}

/// onSignInteract of each sign.
fn sign_interact(cx: &mut SignCx) -> Result<bool, CmdError> {
    let player = cx.player;
    match cx.kind {
        Kind::Balance => {
            let bal = money(cx.ess, player);
            let shown = cur(cx.ess, &bal);
            cx.tl("balance", &[shown]);
            Ok(true)
        }
        Kind::Buy => {
            let mut items = cx.item_trade(1, 2)?;
            let mut charge = cx.trade(3, 1)?;
            if cx.ess.config_bool("allow-bulk-buy-sell", true) && cx.host.sneaking(player) {
                if let (Trade::Item(item), Some(held)) = (&items, held_item(cx.host, player)) {
                    if similar(item, &held) {
                        let initial = item.count;
                        let mut bigger = item.clone();
                        bigger.count = held.count;
                        let each = charge.money().cloned().unwrap_or(Dec::ZERO).div_exact(&Dec::from_i64(initial as i64)).ok_or_else(|| CmdError::Msg("Non-terminating decimal expansion; no exact representable decimal result.".into()))?;
                        charge = Trade::Money(each.mul(&Dec::from_i64(held.count as i64)));
                        items = Trade::Item(bigger);
                    }
                }
            }
            charge.affordable(cx.ess, cx.host, player)?;
            if items.pay(cx.ess, cx.host, player, Overflow::Abort)?.is_some() {
                return Err(sign_err("inventoryFull"));
            }
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Buy", "Interact", Some(&charge), Some(&items));
            Ok(true)
        }
        Kind::Sell => {
            let mut charge = cx.item_trade(1, 2)?;
            let mut paid = cx.trade(3, 1)?;
            if cx.ess.config_bool("allow-bulk-buy-sell", true) && cx.host.sneaking(player) {
                if let (Trade::Item(item), Some(held)) = (&charge, held_item(cx.host, player)) {
                    if similar(item, &held) {
                        let initial = item.count;
                        let mut bigger = item.clone();
                        bigger.count = held.count;
                        let each = paid.money().cloned().unwrap_or(Dec::ZERO).div_exact(&Dec::from_i64(initial as i64)).ok_or_else(|| CmdError::Msg("Non-terminating decimal expansion; no exact representable decimal result.".into()))?;
                        let m = multiplier(cx.ess, cx.host, player);
                        paid = Trade::Money(each.mul(&Dec::from_i64(held.count as i64)).mul(&m));
                        charge = Trade::Item(bigger);
                    }
                }
            }
            charge.affordable(cx.ess, cx.host, player)?;
            paid.pay(cx.ess, cx.host, player, Overflow::Drop)?;
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Sell", "Interact", Some(&charge), Some(&paid));
            Ok(true)
        }
        Kind::Trade => trade_interact(cx),
        Kind::Free => {
            let mut item = cx.item_stack(&cx.line(1), 1)?;
            if item.id == "minecraft:air" {
                return Err(CmdError::Tl("cantSpawnItem", vec![Arg::Plain("Air".into())]));
            }
            item.count = cx.host.max_stack(&item.id);
            let title = if has_custom_name(&item) { custom_name(&item) } else { material(&item.id).to_uppercase() };
            let container = Container { rows: 4, title: container_title(&title), items: vec![Some(item.clone()); 36], tag: "economy:free".into() };
            cx.host.open_container(player, &container);
            cx.log("Free", "Interact", None, Some(&Trade::Item(item)));
            Ok(true)
        }
        Kind::Disposal => {
            let custom = format!("{} {} {}", cx.line(1), cx.line(2), cx.line(3)).trim().to_string();
            let title = if custom.is_empty() { mini_to_legacy(cx.ess, &cx.ess.tl("disposal", &[])) } else { custom };
            let container = Container { rows: 4, title: container_title(&title), items: vec![None; 36], tag: "economy:disposal".into() };
            cx.host.open_container(player, &container);
            Ok(true)
        }
        Kind::Enchant => enchant_interact(cx),
        Kind::Repair => {
            let charge = cx.trade(2, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            let target = cx.line(1);
            if target.eq_ignore_ascii_case("hand") {
                repair_hand(cx)?;
            } else if target.eq_ignore_ascii_case("all") {
                repair_all(cx)?;
            } else {
                return Err(CmdError::Msg(String::new()));
            }
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Repair", "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::Heal => {
            let (health, max) = cx.host.health(player);
            if health == 0.0 {
                return Err(sign_err("healDead"));
            }
            let charge = cx.trade(1, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            cx.host.set_health(player, max);
            let (_, saturation) = cx.host.food(player);
            cx.host.set_food(player, 20, saturation);
            cx.host.set_fire_ticks(player, 0);
            cx.tl("youAreHealed", &[]);
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Heal", "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::GameMode => {
            let charge = cx.trade(2, 1)?;
            let mode = cx.line(1).trim().to_lowercase();
            if mode.is_empty() {
                return Err(CmdError::Tl("invalidSignLine", vec![Arg::from(2)]));
            }
            charge.affordable(cx.ess, cx.host, player)?;
            let gm = if mode.contains("survi") || mode == "0" {
                GameMode::Survival
            } else if mode.contains("creat") || mode == "1" {
                GameMode::Creative
            } else if mode.contains("advent") || mode == "2" {
                GameMode::Adventure
            } else if mode.contains("spec") || mode == "3" {
                GameMode::Spectator
            } else {
                return Err(CmdError::Tl("invalidSignLine", vec![Arg::from(2)]));
            };
            cx.host.set_game_mode(player, gm);
            let now = cx.host.game_mode(player);
            let shown = cx.ess.tl(now.name(), &[]);
            let name = display_name(cx.ess, cx.host, player);
            cx.tl("gameMode", &[Arg::Plain(shown), Arg::Plain(name)]);
            cx.log("gameMode", "Interact", None, Some(&charge));
            charge.charge(cx.ess, cx.host, player)?;
            Ok(true)
        }
        Kind::Info => {
            let charge = cx.trade(3, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            let line = format!("info {} {}", cx.line(1), cx.line(2));
            cx.ess.dispatch(cx.host, Sender::Player(player), line.trim());
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Info", "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::Kit => {
            let kit = cx.line(1).to_lowercase().trim().to_string();
            let group = cx.line(2).trim().to_string();
            let allowed = if group.is_empty() { cx.has(&format!("essentials.kits.{kit}")) } else { group == "§2Everyone" || cx.in_group(&group) };
            if !allowed {
                return Err(if group.is_empty() { CmdError::Tl("noKitPermission", vec![Arg::Plain(format!("essentials.kits.{kit}"))]) } else { CmdError::Tl("noKitGroup", vec![Arg::Plain(group)]) });
            }
            let charge = cx.trade(3, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            // Kits belong to the items area: the kit command does the delay, the items and kitReceive.
            cx.ess.dispatch(cx.host, Sender::Player(player), &format!("kit {kit}"));
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Kit", "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::Mail => {
            cx.ess.dispatch(cx.host, Sender::Player(player), "mail read");
            Ok(true)
        }
        Kind::RandomTeleport => {
            let name = cx.line(1).trim().to_string();
            let location = if name.is_empty() { "default".to_string() } else { name };
            let who = user_name(cx.ess, cx.host, player);
            cx.ess.dispatch(cx.host, Sender::Console, &format!("tpr {location} {who}"));
            Ok(true)
        }
        Kind::Warp => {
            let warp = cx.line(1);
            let group = cx.line(2);
            if !group.is_empty() {
                if group != "§2Everyone" && !cx.in_group(&group) {
                    return Err(sign_err("warpUsePermission"));
                }
            } else if cx.ess.config_bool("per-warp-permission", false) && !cx.has(&format!("essentials.warps.{warp}")) {
                return Err(sign_err("warpUsePermission"));
            }
            let charge = cx.trade(3, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            let who = user_name(cx.ess, cx.host, player);
            cx.ess.dispatch(cx.host, Sender::Console, &format!("warp {warp} {who}"));
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Warp", "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::Anvil | Kind::Cartography | Kind::Grindstone | Kind::Loom | Kind::Smithing | Kind::Workbench => {
            if let Some(station) = station_of(cx.kind) {
                cx.host.open_station(player, station);
            }
            Ok(true)
        }
        Kind::Spawnmob => {
            let charge = cx.trade(3, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            let who = user_name(cx.ess, cx.host, player);
            let line = format!("spawnmob {} {} {who}", cx.line(2).trim(), cx.line(1).trim());
            cx.ess.dispatch(cx.host, Sender::Console, &line);
            charge.charge(cx.ess, cx.host, player)?;
            cx.log("Spawnmob", "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::Time => {
            let world = cx.host.location(player).map(|l| l.world).unwrap_or_else(|| cx.world.clone());
            if cx.line(1).is_empty() && cx.line(2).is_empty() && cx.line(3).is_empty() {
                let now = cx.host.world_time(&world);
                let shown = Arg::Parsed(format_ticks(cx.ess, now));
                cx.tl("timeWorldCurrentSign", &[shown]);
                return Ok(true);
            }
            let charge = cx.trade(2, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            let value = cx.line(1);
            let mut time = cx.host.world_time(&world);
            time -= time % 24000;
            let (target, sub) = if value.eq_ignore_ascii_case("§2Day") {
                (time + 24000, "TimeDay")
            } else if value.eq_ignore_ascii_case("§2Night") {
                (time + 37700, "TimeNight")
            } else {
                return Err(sign_err("onlyDayNight"));
            };
            cx.host.set_world_time(&world, target);
            charge.charge(cx.ess, cx.host, player)?;
            cx.log(sub, "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::Weather => {
            let world = cx.host.location(player).map(|l| l.world).unwrap_or_else(|| cx.world.clone());
            if cx.line(1).is_empty() && cx.line(2).is_empty() && cx.line(3).is_empty() {
                let storm = cx.host.weather(&world).0;
                cx.tl(if storm { "weatherSignStorm" } else { "weatherSignSun" }, &[]);
                return Ok(true);
            }
            let charge = cx.trade(2, 1)?;
            charge.affordable(cx.ess, cx.host, player)?;
            let value = cx.line(1);
            let (storm, sub) = if value.eq_ignore_ascii_case("§2Sun") {
                (false, "WeatherSun")
            } else if value.eq_ignore_ascii_case("§2Storm") {
                (true, "WeatherStorm")
            } else {
                return Err(sign_err("onlySunStorm"));
            };
            cx.host.set_weather(&world, storm, false, None);
            charge.charge(cx.ess, cx.host, player)?;
            cx.log(sub, "Interact", None, Some(&charge));
            Ok(true)
        }
        Kind::Protection => Ok(true),
    }
}

/// DescParseTickFormat.format: `12:00 or 12:00 PM or 6000 ticks`.
fn format_ticks(ess: &Ess, ticks: i64) -> String {
    let mut t = (ticks - 18000 + 24000).rem_euclid(24000);
    let hours = t / 1000;
    t -= hours * 1000;
    let minutes = (t as f64 / (1000.0 / 60.0)).floor() as i64;
    let h12 = if hours % 12 == 0 { 12 } else { hours % 12 };
    let ampm = if hours < 12 { "AM" } else { "PM" };
    ess.tl(
        "timeFormat",
        &[Arg::Plain(format!("{hours:02}:{minutes:02}")), Arg::Plain(format!("{h12}:{minutes:02} {ampm}")), Arg::Plain(format!("{} ticks", ticks.rem_euclid(24000)))],
    )
}

fn enchant_interact(cx: &mut SignCx) -> Result<bool, CmdError> {
    let player = cx.player;
    let hand = held_item(cx.host, player);
    let item_name = cx.line(1);
    let any = item_name == "*" || item_name.eq_ignore_ascii_case("any");
    let search = if any { None } else { Some(cx.item_stack(&item_name, 1)?) };
    let charge = cx.trade(3, 1)?;
    charge.affordable(cx.ess, cx.host, player)?;
    let line = cx.line(2);
    let parts: Vec<&str> = line.split(':').collect();
    let (id, max) = find_enchantment(cx.host, parts[0]).ok_or(sign_err("enchantmentNotFound"))?;
    let mut level: i64 = 1;
    if parts.len() > 1 {
        level = parts[1].trim().parse::<i32>().map_err(|_| wrapped(cx.ess, CmdError::Msg(format!("For input string: \"{}\"", parts[1]))))? as i64;
    }
    let Some(mut hand) = hand.filter(|h| h.count == 1 && enchant_level(h, &id) != Some(level)) else {
        return Err(CmdError::Tl("missingItems", vec![Arg::from(1), Arg::Plain(item_name)]));
    };
    if let Some(s) = &search {
        if s.id != hand.id {
            return Err(CmdError::Tl("missingItems", vec![Arg::from(1), Arg::Plain(material(&s.id).replace('_', " "))]));
        }
    }
    let unsafe_ok = cx.ess.config_bool("allow-unsafe-enchantments", false) && cx.has("essentials.signs.enchant.allowunsafe");
    if level > max as i64 && !unsafe_ok {
        return Err(wrapped(cx.ess, CmdError::Msg(format!("Enchantment level is either too low or too high (given {level}, bounds are 1 to {max})"))));
    }
    if !hand.components.is_object() {
        hand.components = json!({});
    }
    let comps = hand.components.as_object_mut().expect("object");
    let enchants = comps.entry("enchantments").or_insert_with(|| json!({}));
    if !enchants.is_object() {
        *enchants = json!({});
    }
    let map = enchants.as_object_mut().expect("object");
    if level == 0 {
        map.remove(&id);
        if map.is_empty() {
            comps.remove("enchantments");
        }
    } else {
        map.insert(id.clone(), json!(level));
    }
    set_held(cx.host, player, Some(hand));
    let real = material(&id).replace('_', " ");
    cx.tl(if level == 0 { "enchantmentRemoved" } else { "enchantmentApplied" }, &[Arg::Plain(real)]);
    charge.charge(cx.ess, cx.host, player)?;
    cx.log("Enchant", "Interact", Some(&charge), Some(&charge));
    Ok(true)
}

fn is_enchanted(item: &Item) -> bool {
    item.components.get("enchantments").and_then(Value::as_object).is_some_and(|m| !m.is_empty())
}
fn repair_charge(item: &Item) -> Trade {
    Trade::Command(format!("repair-{}", material(&item.id).replace('_', "-")), Some(Box::new(Trade::Command("repair-item".into(), None))))
}
fn repaired(mut item: Item) -> Item {
    if let Some(o) = item.components.as_object_mut() {
        o.remove("damage");
        o.remove("minecraft:damage");
    }
    item
}
/// Commandrepair.repairHand.
fn repair_hand(cx: &mut SignCx) -> Result<(), CmdError> {
    let player = cx.player;
    let item = held_item(cx.host, player);
    let Some(item) = item.filter(|i| !cx.host.item_is_block(&i.id) && damage(i) != 0) else { return Err(sign_err("repairInvalidType")) };
    if is_enchanted(&item) && !cx.ess.config_bool("repair-enchanted", true) && !cx.has("essentials.repair.enchanted") {
        return Err(sign_err("repairEnchanted"));
    }
    let charge = repair_charge(&item);
    charge.affordable(cx.ess, cx.host, player)?;
    if cx.host.max_damage(&item.id) < 1 {
        return Err(sign_err("repairInvalidType"));
    }
    let name = material(&item.id).replace('_', " ");
    set_held(cx.host, player, Some(repaired(item)));
    charge.charge(cx.ess, cx.host, player)?;
    cx.tl("repair", &[Arg::Plain(name)]);
    Ok(())
}
/// Commandrepair.repairAll.
fn repair_all(cx: &mut SignCx) -> Result<(), CmdError> {
    let player = cx.player;
    let mut slots: Vec<usize> = STORAGE.collect();
    if cx.has("essentials.repair.armor") {
        slots.extend(36..40);
    }
    let mut names = Vec::new();
    let inv = cx.host.inventory(player);
    for slot in slots {
        let Some(Some(item)) = inv.get(slot).cloned() else { continue };
        if cx.host.item_is_block(&item.id) || damage(&item) == 0 {
            continue;
        }
        let charge = repair_charge(&item);
        if let Err(e) = charge.affordable(cx.ess, cx.host, player) {
            let me = cx.me();
            show_error(cx.ess, cx.host, me, e);
            continue;
        }
        if is_enchanted(&item) && !cx.ess.config_bool("repair-enchanted", true) && !cx.has("essentials.repair.enchanted") {
            continue;
        }
        if cx.host.max_damage(&item.id) < 1 {
            continue;
        }
        let name = material(&item.id).replace('_', " ");
        cx.host.set_slot(player, slot, Some(repaired(item)));
        let _ = charge.charge(cx.ess, cx.host, player);
        names.push(name);
    }
    if names.is_empty() {
        return Err(sign_err("repairNone"));
    }
    cx.tl("repair", &[Arg::Plain(names.join(", "))]);
    Ok(())
}

/// onSignBreak of each sign.
fn sign_break(cx: &mut SignCx) -> Result<bool, CmdError> {
    match cx.kind {
        Kind::Trade => trade_break(cx),
        Kind::Protection => {
            let lines = cx.lines.clone();
            Ok(prot_check_sign(cx.ess, cx.host, &lines, Some(cx.player)) == Prot::Owner)
        }
        _ => Ok(true),
    }
}

// ----- [Trade] (SignTrade) -------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Amt {
    Total,
    Rounded,
    Cost,
}

fn ts_full(line: &str) -> Result<(), CmdError> {
    if line.chars().count() > 15 { Err(sign_err("tradeSignFull")) } else { Ok(()) }
}
fn is_exp(s: &str) -> bool {
    s.eq_ignore_ascii_case("exp") || s.eq_ignore_ascii_case("xp")
}

fn ts_validate(cx: &mut SignCx, index: usize, amount_needed: bool) -> Result<(), CmdError> {
    let line = cx.line(index).trim().to_string();
    if line.is_empty() {
        return Err(CmdError::Tl("emptySignLine", vec![Arg::from(index + 1)]));
    }
    let split = split_sign(&line, None);
    if split.len() == 1 && !amount_needed {
        if let Some(m) = cx.money_line(&split[0])? {
            let nl = format!("{}:0", short_currency(cx.ess, &m));
            ts_full(&nl)?;
            cx.set(index, nl);
            return Ok(());
        }
    }
    if split.len() == 2 && amount_needed {
        let money = cx.money_line(&split[0])?;
        let amount = cx.big_decimal_positive(&split[1])?;
        if let Some(m) = money {
            let amount = amount.sub(&amount.rem(&m));
            let min = Dec::parse("0.01").expect("decimal");
            if amount.cmp(&min) == Ordering::Less || m.cmp(&min) == Ordering::Less {
                return Err(sign_err("moreThanZero"));
            }
            let nl = format!("{}:{}", short_currency(cx.ess, &m), format_as_currency(&amount));
            ts_full(&nl)?;
            cx.set(index, nl);
            return Ok(());
        }
    }
    if split.len() == 2 && !amount_needed {
        let amount = SignCx::integer_positive(&split[0])?;
        if !is_exp(&split[1]) && cx.item_stack(&split[1], amount)?.id == "minecraft:air" {
            return Err(sign_err("moreThanZero"));
        }
        let nl = format!("{amount} {}:0", split[1]);
        ts_full(&nl)?;
        cx.set(index, nl);
        return Ok(());
    }
    if split.len() == 3 && amount_needed {
        let stack = SignCx::integer_positive(&split[0])?;
        let mut amount = SignCx::integer_positive(&split[2])?;
        amount -= amount % stack;
        if amount < 1 || stack < 1 {
            return Err(sign_err("moreThanZero"));
        }
        if !is_exp(&split[1]) && cx.item_stack(&split[1], stack)?.id == "minecraft:air" {
            return Err(sign_err("moreThanZero"));
        }
        let nl = format!("{stack} {}:{amount}", split[1]);
        ts_full(&nl)?;
        cx.set(index, nl);
        return Ok(());
    }
    Err(CmdError::Tl("invalidSignLine", vec![Arg::from(index + 1)]))
}

fn ts_get(cx: &mut SignCx, index: usize, amt: Amt, not_empty: bool) -> Result<Trade, CmdError> {
    let line = cx.line(index).trim().to_string();
    if line.is_empty() {
        return Err(CmdError::Tl("emptySignLine", vec![Arg::from(index + 1)]));
    }
    let split = split_sign(&line, None);
    if split.len() == 2 {
        let parsed = (|| -> Result<(Option<Dec>, Dec), CmdError> {
            let money = cx.money_line(&split[0])?;
            let amount = if not_empty { cx.big_decimal_positive(&split[1])? } else { cx.big_decimal(&split[1])? };
            Ok((money, amount))
        })();
        match parsed {
            Err(_) => return Err(sign_err("tradeSignEmpty")),
            Ok((Some(m), amount)) => return Ok(Trade::Money(if amt == Amt::Cost { m } else { amount })),
            Ok((None, _)) => {}
        }
    }
    if split.len() == 3 {
        let stack = SignCx::integer_positive(&split[0])?;
        let mut amount = SignCx::integer(&split[2])?;
        if amt == Amt::Rounded {
            amount -= amount % stack;
        }
        if is_exp(&split[1]) {
            if not_empty && (amount < 1 || stack < 1) {
                return Err(sign_err("tradeSignEmpty"));
            }
            return Ok(Trade::Exp(if amt == Amt::Cost { stack } else { amount }));
        }
        let mut item = cx.item_stack(&split[1], stack)?;
        if not_empty && (amount < 1 || stack < 1 || item.id == "minecraft:air" || amount < stack) {
            return Err(sign_err("tradeSignEmpty"));
        }
        item.count = (if amt == Amt::Cost { stack } else { amount }) as i32;
        return Ok(Trade::Item(item));
    }
    Err(CmdError::Tl("invalidSignLine", vec![Arg::from(index + 1)]))
}

fn ts_set_amount(cx: &mut SignCx, index: usize, value: &Dec, validation: bool) -> Result<(), CmdError> {
    let line = cx.line(index).trim().to_string();
    if line.is_empty() {
        return Err(CmdError::Tl("emptySignLine", vec![Arg::from(index + 1)]));
    }
    let split = split_sign(&line, None);
    if split.len() == 2 {
        let money = cx.money_line(&split[0])?;
        cx.big_decimal(&split[1])?;
        if let Some(m) = money {
            let nl = format!("{}:{}", short_currency(cx.ess, &m), format_as_currency(value));
            ts_full(&nl)?;
            if !validation {
                cx.set(index, nl);
            }
            return Ok(());
        }
    }
    if split.len() == 3 {
        let stack = SignCx::integer_positive(&split[0])?;
        if !is_exp(&split[1]) {
            cx.item_stack(&split[1], stack)?;
        }
        let nl = format!("{stack} {}:{}", split[1], value.trunc_i64());
        ts_full(&nl)?;
        if !validation {
            cx.set(index, nl);
        }
        return Ok(());
    }
    Err(CmdError::Tl("invalidSignLine", vec![Arg::from(index + 1)]))
}

fn ts_change(cx: &mut SignCx, index: usize, by: &Dec, validation: bool) -> Result<(), CmdError> {
    let line = cx.line(index).trim().to_string();
    if line.is_empty() {
        return Err(CmdError::Tl("emptySignLine", vec![Arg::from(index + 1)]));
    }
    let split = split_sign(&line, None);
    let current = match split.len() {
        2 => cx.big_decimal(&split[1])?,
        3 => cx.big_decimal(&split[2])?,
        _ => return Err(CmdError::Tl("invalidSignLine", vec![Arg::from(index + 1)])),
    };
    ts_set_amount(cx, index, &current.add(by), validation)
}
fn trade_amount(t: &Trade) -> Dec {
    match t {
        Trade::Money(m) => m.clone(),
        Trade::Item(i) => Dec::from_i64(i.count as i64),
        Trade::Exp(e) => Dec::from_i64(*e),
        Trade::Command(..) => Dec::ZERO,
    }
}
fn ts_add(cx: &mut SignCx, index: usize, t: &Trade, validation: bool) -> Result<(), CmdError> {
    ts_change(cx, index, &trade_amount(t), validation)
}
fn ts_sub(cx: &mut SignCx, index: usize, t: &Trade, validation: bool) -> Result<(), CmdError> {
    ts_change(cx, index, &trade_amount(t).neg(), validation)
}
/// EssentialsSign.isOwner without a sign data provider: the name on the line.
fn is_owner(cx: &SignCx, index: usize) -> bool {
    strip_color(&cx.lines[index]).eq_ignore_ascii_case(&cx.username)
}

fn trade_interact(cx: &mut SignCx) -> Result<bool, CmdError> {
    let player = cx.player;
    if is_owner(cx, 3) {
        let store = trade_recharge(cx)?;
        let stored = ts_get(cx, 1, Amt::Total, true).and_then(|stored| ts_sub(cx, 1, &stored, false).map(|_| stored));
        match stored {
            Ok(stored) => {
                if let Some(left) = stored.pay(cx.ess, cx.host, player, Overflow::Return)? {
                    ts_set_amount(cx, 1, &Dec::from_i64(left.count as i64), false)?;
                }
                cx.log("Trade", "Withdraw", Some(&stored), None);
            }
            Err(_) if store.is_none() => return Err(sign_err("tradeSignEmptyOwner")),
            Err(_) => {}
        }
        if let Some(s) = &store {
            cx.log("Trade", "Deposit", Some(s), None);
        }
    } else {
        let charge = ts_get(cx, 1, Amt::Cost, false)?;
        let trade = ts_get(cx, 2, Amt::Cost, true)?;
        charge.affordable(cx.ess, cx.host, player)?;
        ts_add(cx, 1, &charge, true)?;
        ts_sub(cx, 2, &trade, true)?;
        ts_add(cx, 1, &charge, false)?;
        ts_sub(cx, 2, &trade, false)?;
        if trade.pay(cx.ess, cx.host, player, Overflow::Abort)?.is_some() {
            ts_sub(cx, 1, &charge, false)?;
            ts_add(cx, 2, &trade, false)?;
            return Err(sign_err("inventoryFull"));
        }
        charge.charge(cx.ess, cx.host, player)?;
        cx.log("Trade", "Interact", Some(&charge), Some(&trade));
    }
    Ok(true)
}

/// SignTrade.rechargeSign: the owner holding the traded item adds one batch of it to the stock.
fn trade_recharge(cx: &mut SignCx) -> Result<Option<Trade>, CmdError> {
    let trade = ts_get(cx, 2, Amt::Cost, false)?;
    let Trade::Item(wanted) = &trade else { return Ok(None) };
    let Some(stack) = held_item(cx.host, cx.player) else { return Ok(None) };
    let enchants = |i: &Item| i.components.get("enchantments").cloned();
    if stack.id == "minecraft:air" || stack.id != wanted.id || damage(&stack) != damage(wanted) || enchants(&stack) != enchants(wanted) {
        return Ok(None);
    }
    if stack.id.ends_with("potion") && !similar(&stack, wanted) {
        return Ok(None);
    }
    let amount = wanted.count;
    if !contains_at_least(cx.host, cx.player, wanted, amount) {
        return Ok(None);
    }
    let mut batch = stack.clone();
    batch.count = amount;
    let store = Trade::Item(batch);
    ts_add(cx, 2, &store, true)?;
    store.charge(cx.ess, cx.host, cx.player)?;
    ts_add(cx, 2, &store, false)?;
    Ok(Some(store))
}

fn trade_break(cx: &mut SignCx) -> Result<bool, CmdError> {
    let owner = is_owner(cx, 3);
    let can_break = owner || cx.has("essentials.signs.trade.override");
    let can_collect = owner || cx.has("essentials.signs.trade.override.collect");
    if !can_break {
        return Ok(false);
    }
    let stored = ts_get(cx, 1, Amt::Total, false).and_then(|a| ts_get(cx, 2, Amt::Total, false).map(|b| (a, b)));
    let (stored1, stored2) = match stored {
        Ok(s) => s,
        Err(_) if cx.has("essentials.signs.trade.override") => return Ok(true),
        Err(e) => return Err(e),
    };
    if !can_collect {
        cx.log("Trade", "Destroy", Some(&stored2), Some(&stored1));
        return Ok(true);
    }
    stored1.pay(cx.ess, cx.host, cx.player, Overflow::Drop)?;
    stored2.pay(cx.ess, cx.host, cx.player, Overflow::Drop)?;
    cx.log("Trade", "Break", Some(&stored2), Some(&stored1));
    Ok(true)
}

// ----- [Protection] (SignProtection) ---------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum Prot {
    NotAllowed,
    Allowed,
    NoSign,
    Owner,
}

fn adjacent(pos: [i32; 3]) -> [[i32; 3]; 6] {
    [rel(pos, NORTH), rel(pos, SOUTH), rel(pos, EAST), rel(pos, WEST), rel(pos, DOWN), rel(pos, UP)]
}
fn has_adjacent_protected(host: &mut dyn Host, world: &str, pos: [i32; 3]) -> bool {
    adjacent(pos).into_iter().any(|b| host.block(world, b).is_some_and(|s| is_protected_block(&block_material(&s))))
}
fn prot_check_sign(ess: &mut Ess, host: &mut dyn Host, lines: &[String; 4], user: Option<Uuid>) -> Prot {
    let Some(user) = user else { return Prot::NotAllowed };
    if ess.has(host, Sender::Player(user), "essentials.signs.protection.override") {
        return Prot::Owner;
    }
    let name = host.name(user).unwrap_or_default();
    let username: String = name.chars().take(13).collect();
    if strip_color(&lines[3]).eq_ignore_ascii_case(&username) {
        return Prot::Owner;
    }
    for line in &lines[1..=2] {
        if line.starts_with('(') && line.ends_with(')') && line.len() >= 2 {
            let group = &line[1..line.len() - 1];
            if ess.perms.groups_of(user, &name).iter().any(|g| g.eq_ignore_ascii_case(group)) {
                return Prot::Allowed;
            }
        } else if line.eq_ignore_ascii_case(&username) {
            return Prot::Allowed;
        }
    }
    Prot::NotAllowed
}
fn prot_check_block(ess: &mut Ess, host: &mut dyn Host, world: &str, pos: [i32; 3], user: Option<Uuid>) -> Prot {
    match sign_at(host, world, pos) {
        Some(lines) if lines[0] == success_name(ess, Kind::Protection) => prot_check_sign(ess, host, &lines, user),
        _ => Prot::NoSign,
    }
}
fn connected_signs(ess: &mut Ess, host: &mut dyn Host, world: &str, pos: [i32; 3], user: Option<Uuid>, depth: i32, out: &mut Vec<([i32; 3], Prot)>) {
    for b in adjacent(pos) {
        if out.iter().any(|(p, _)| *p == b) {
            continue;
        }
        let check = prot_check_block(ess, host, world, b, user);
        out.push((b, check));
        if depth > 0 && host.block(world, b).is_some_and(|s| is_protected_block(&block_material(&s))) {
            connected_signs(ess, host, world, b, user, depth - 1, out);
        }
    }
}
/// SignProtection.isBlockProtected.
fn is_block_protected(ess: &mut Ess, host: &mut dyn Host, world: &str, pos: [i32; 3], user: Option<Uuid>, secure: bool) -> Prot {
    let mut signs = Vec::new();
    connected_signs(ess, host, world, pos, user, if secure { 4 } else { 2 }, &mut signs);
    let mut state = Prot::NoSign;
    for (_, s) in &signs {
        if *s == Prot::Allowed {
            state = *s;
        } else if *s == Prot::NotAllowed && state != Prot::Allowed {
            state = *s;
        }
    }
    if (!secure || state == Prot::NoSign) && signs.iter().any(|(_, s)| *s == Prot::Owner) {
        return Prot::Owner;
    }
    state
}

// ----- Listeners (SignBlockListener, SignPlayerListener, SignEntityListener) ---------------------------------------

fn sign_cx<'a>(ess: &'a mut Ess, host: &'a mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], lines: [String; 4], kind: Kind) -> SignCx<'a> {
    let name = host.name(player).unwrap_or_default();
    SignCx { username: name.chars().take(13).collect(), ess, host, player, world: world.to_string(), pos, lines, kind }
}
fn may(ess: &mut Ess, host: &mut dyn Host, player: Uuid, kind: Kind, action: &str) -> bool {
    let l = kind.lower();
    ess.has(host, Sender::Player(player), &format!("essentials.signs.{l}.{action}")) || ess.has(host, Sender::Player(player), &format!("essentials.signs.{action}.{l}"))
}

/// EssentialsSign.onSignCreate(SignChangeEvent): false cancels the edit.
fn create_sign(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], lines: &mut [String; 4], kind: Kind) -> bool {
    if !may(ess, host, player, kind, "create") {
        return true;
    }
    lines[0] = fail_name(ess, kind);
    let mut cx = sign_cx(ess, host, player, world, pos, lines.clone(), kind);
    let result = sign_create(&mut cx);
    *lines = cx.lines;
    match result {
        Ok(true) => {
            lines[0] = success_name(ess, kind);
            true
        }
        Ok(false) => false,
        Err(e) => {
            show_error(ess, host, Sender::Player(player), e);
            true
        }
    }
}

/// EssentialsSign.onSignInteract(Block, Player): throttle, permission, then the sign.
fn use_sign(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], lines: [String; 4], kind: Kind) -> bool {
    let now = host.now_millis();
    let per_second = ess.config_i64("sign-use-per-second", 4);
    let per_second = if per_second > 0 { per_second } else { 1 };
    let last = ess.state::<EcoState>().throttle.get(&player).copied();
    if last.is_some_and(|l| now < l + 1000 / per_second) {
        return false;
    }
    ess.state::<EcoState>().throttle.insert(player, now);
    if host.health(player).0 <= 0.0 || !may(ess, host, player, kind, "use") {
        return false;
    }
    let before = lines.clone();
    let mut cx = sign_cx(ess, host, player, world, pos, lines, kind);
    let result = sign_interact(&mut cx);
    let after = cx.lines.clone();
    if after != before {
        host.set_sign_lines(world, pos, &after);
    }
    match result {
        Ok(r) => r,
        Err(e) => {
            show_error(ess, host, Sender::Player(player), e);
            false
        }
    }
}

/// EssentialsSign.onSignBreak(Block, Player).
fn break_sign(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], lines: [String; 4], kind: Kind) -> bool {
    if !may(ess, host, player, kind, "break") {
        return false;
    }
    let before = lines.clone();
    let mut cx = sign_cx(ess, host, player, world, pos, lines, kind);
    let result = sign_break(&mut cx);
    let after = cx.lines.clone();
    if after != before {
        host.set_sign_lines(world, pos, &after);
    }
    match result {
        Ok(r) => r,
        Err(e) => {
            show_error(ess, host, Sender::Player(player), e);
            false
        }
    }
}

/// SignBlockListener.onSignSignChange2 (colours, the first line normalized) and onSignSignChange (creation).
fn on_sign_change(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], lines: &mut [String; 4]) -> Allow {
    let (kinds, on) = enabled_signs(ess);
    if !on {
        return Allow::Yes;
    }
    for line in lines.iter_mut() {
        *line = format_sign_line(ess, host, player, line);
    }
    let top = strip_color(&lines[0]).to_lowercase().trim().to_string();
    if !top.is_empty() {
        let unprotected = unprotected_signs(ess);
        for kind in KINDS {
            let success = strip_color(&success_name(ess, kind).to_lowercase());
            if top.contains(&success) {
                if !kinds.contains(&kind) && unprotected.contains(&kind) {
                    continue;
                }
                lines[0] = top.clone();
            }
        }
    }
    for kind in kinds {
        if lines[0].to_lowercase() == success_name(ess, kind).to_lowercase() {
            return Allow::No;
        }
        if lines[0].to_lowercase() == template_name(ess, kind).to_lowercase() && !create_sign(ess, host, player, world, pos, lines, kind) {
            return Allow::No;
        }
    }
    Allow::Yes
}

/// SignPlayerListener.onSignPlayerInteract: a right click on an Essentials sign uses it (and never edits it);
/// on a guarded block, the [Protection] rules apply.
fn on_interact(ess: &mut Ess, host: &mut dyn Host, player: Uuid, block: Option<(&str, [i32; 3], &str)>, _held: Option<&Item>) -> Allow {
    let (kinds, on) = enabled_signs(ess);
    let Some((world, pos, state)) = block.filter(|_| on) else { return Allow::Yes };
    let mat = block_material(state);
    if is_sign_block(&mat) {
        let Some(lines) = host.sign_lines(world, pos).map(|l| l.map(|s| clean_line(&s))) else { return Allow::Yes };
        for kind in kinds {
            if lines[0].to_lowercase() == success_name(ess, kind).to_lowercase() {
                use_sign(ess, host, player, world, pos, lines, kind);
                return Allow::No;
            }
        }
        return Allow::Yes;
    }
    for kind in kinds {
        if kind.guards(&mat) && !protection_block_interact(ess, host, player, world, pos, &mat) {
            return Allow::No;
        }
    }
    Allow::Yes
}

fn protection_block_interact(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], mat: &str) -> bool {
    let state = is_block_protected(ess, host, world, pos, Some(player), false);
    if matches!(state, Prot::Owner | Prot::NoSign | Prot::Allowed) {
        return true;
    }
    if state == Prot::NotAllowed && ess.has(host, Sender::Player(player), "essentials.signs.protection.override") {
        return true;
    }
    ess.send_tl(host, Sender::Player(player), "noAccessPermission", &[Arg::Plain(mat.to_string())]);
    false
}

/// SignBlockListener.protectSignsAndBlocks.
fn on_block_break(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], state: &str) -> Allow {
    let (kinds, on) = enabled_signs(ess);
    if !on {
        return Allow::Yes;
    }
    if block_holds_signs(host, world, pos) {
        return Allow::No;
    }
    let mat = block_material(state);
    if is_sign_block(&mat) {
        if let Some(lines) = host.sign_lines(world, pos).map(|l| l.map(|s| clean_line(&s))) {
            for kind in kinds.iter().copied() {
                if lines[0].to_lowercase() == success_name(ess, kind).to_lowercase() && !break_sign(ess, host, player, world, pos, lines.clone(), kind) {
                    return Allow::No;
                }
            }
        }
    }
    for kind in kinds {
        if kind.guards(&mat) {
            let state = is_block_protected(ess, host, world, pos, Some(player), false);
            let ok = matches!(state, Prot::Owner | Prot::NoSign) || ess.has(host, Sender::Player(player), "essentials.signs.protection.override");
            if !ok {
                ess.send_tl(host, Sender::Player(player), "noDestroyPermission", &[Arg::Plain(mat.clone())]);
                return Allow::No;
            }
        }
    }
    Allow::Yes
}

/// SignBlockListener.onSignBlockPlace: no chest/furnace/dispenser next to someone else's protection.
fn on_block_place(ess: &mut Ess, host: &mut dyn Host, player: Uuid, world: &str, pos: [i32; 3], state: &str) -> Allow {
    let (kinds, on) = enabled_signs(ess);
    let mat = block_material(state);
    if !on || is_sign_block(&mat) {
        return Allow::Yes;
    }
    for kind in kinds {
        if kind.guards(&mat) {
            for adj in adjacent(pos) {
                let s = is_block_protected(ess, host, world, adj, Some(player), true);
                if matches!(s, Prot::Allowed | Prot::NotAllowed) && !ess.has(host, Sender::Player(player), "essentials.signs.protection.override") {
                    ess.send_tl(host, Sender::Player(player), "noPlacePermission", &[Arg::Plain(mat.clone())]);
                    return Allow::No;
                }
            }
        }
    }
    Allow::Yes
}

/// Explosions, burning, fire spread and pistons (SignBlockListener/SignEntityListener): Essentials signs, the
/// blocks holding them and protected blocks survive.
fn on_world(ess: &mut Ess, host: &mut dyn Host, kind: &str, world: &str, pos: [i32; 3]) -> Allow {
    if kind.starts_with("spawn:") || kind == "flow" {
        return Allow::Yes;
    }
    let (kinds, on) = enabled_signs(ess);
    if !on {
        return Allow::Yes;
    }
    if let Some(lines) = sign_at(host, world, pos) {
        let name = strip_color(&lines[0]).replace(|c: char| !c.is_ascii_alphabetic(), "");
        if lines[0].starts_with("§1[") && lines[0].ends_with(']') && kinds.iter().any(|k| k.name().eq_ignore_ascii_case(&name)) {
            return Allow::No;
        }
    }
    if block_holds_signs(host, world, pos) {
        return Allow::No;
    }
    let mat = host.block(world, pos).map(|s| block_material(&s)).unwrap_or_default();
    for k in kinds {
        if k.guards(&mat) && is_block_protected(ess, host, world, pos, None, false) != Prot::NoSign {
            return Allow::No;
        }
    }
    Allow::Yes
}

#[cfg(test)]
mod tests {
    // L1 tests of the economy area (included by ess_economy.rs as its `tests` module).

    use super::*;
    use crate::game::ess_testkit::*;
    use serde_json::{Value, json};

    const W: &str = "minecraft:overworld";

    fn money(t: &mut TestServer, p: Uuid) -> String {
        match t.ess.users.user(p).get("money") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => "unset".into(),
        }
    }
    fn set_money(t: &mut TestServer, p: Uuid, amount: &str) {
        t.ess.users.user(p).set("money", json!(amount));
    }
    fn give(t: &mut TestServer, p: Uuid, slot: usize, id: &str, count: i32) {
        t.host.p(p).inventory[slot] = Some(Item::new(id, count));
    }
    fn count(t: &mut TestServer, p: Uuid, id: &str) -> i32 {
        let id = if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") };
        t.host.p(p).inventory.iter().flatten().filter(|i| i.id == id).map(|i| i.count).sum()
    }
    /// A player finishes writing a sign at `pos`; the host keeps the lines when the edit is allowed.
    fn write_sign(t: &mut TestServer, p: Uuid, pos: [i32; 3], lines: [&str; 4]) -> (Allow, [String; 4]) {
        let mut l = lines.map(String::from);
        t.host.blocks.insert((W.into(), pos), "minecraft:oak_sign[rotation=0,waterlogged=false]".into());
        let r = t.ess.on_sign_change(&mut t.host, p, W, pos, &mut l);
        if r == Allow::Yes {
            t.host.signs.insert((W.into(), pos), l.clone());
        }
        (r, l)
    }
    /// A right click on the block at `pos`; returns the decision and what the player was told.
    fn click(t: &mut TestServer, p: Uuid, pos: [i32; 3]) -> (Allow, Vec<String>) {
        t.host.now += 1000;
        t.said(p);
        let state = t.host.block(W, pos).unwrap();
        let slot = t.host.p(p).held;
        let held = t.host.p(p).inventory[slot].clone();
        let r = t.ess.on_interact(&mut t.host, p, Some((W, pos, &state)), held.as_ref());
        (r, t.said(p))
    }
    fn break_at(t: &mut TestServer, p: Uuid, pos: [i32; 3]) -> (Allow, Vec<String>) {
        t.said(p);
        let state = t.host.block(W, pos).unwrap();
        let r = t.ess.on_block_break(&mut t.host, p, W, pos, &state);
        (r, t.said(p))
    }
    fn sign(t: &mut TestServer, pos: [i32; 3]) -> [String; 4] {
        t.host.signs.get(&(W.to_string(), pos)).cloned().expect("a sign")
    }
    fn signs_config(extra: &str) -> TestServer {
        TestServer::with_config(&format!(
            "enabledSigns: [color, balance, buy, sell, trade, free, disposal, enchant, repair, heal, gamemode, info, kit, mail, randomteleport, warp, anvil, cartography, grindstone, loom, smithing, workbench, spawnmob, time, weather, protection]\n{extra}"
        ))
    }

    // ===== Decimal and formatting ======================================================================

    #[test]
    fn decimals_are_exact() {
        let a = Dec::parse("0.1").unwrap();
        let three = a.add(&a).add(&a);
        assert_eq!(three.to_string(), "0.3");
        assert_eq!(Dec::parse("10.50").unwrap().to_string(), "10.50");
        assert_eq!(Dec::parse("-3").unwrap().mul(&Dec::parse("1.5").unwrap()).to_string(), "-4.5");
        assert_eq!(Dec::parse("10").unwrap().sub(&Dec::parse("0.01").unwrap()).to_string(), "9.99");
        assert!(Dec::parse("2.0").unwrap().cmp(&Dec::parse("2").unwrap()).is_eq());
        assert_eq!(Dec::parse("1").unwrap().div_exact(&Dec::parse("4").unwrap()).unwrap().to_string(), "0.25");
        assert!(Dec::parse("10").unwrap().div_exact(&Dec::parse("3").unwrap()).is_none());
        assert!(Dec::parse("abc").is_none());
    }

    #[test]
    fn currency_formatting_follows_number_util() {
        let mut t = TestServer::new();
        let d = |s: &str| Dec::parse(s).unwrap();
        let show = |t: &TestServer, s: &str| crate::game::ess_text::plain(&t.ess.render(&display_currency(&t.ess, &d(s))));
        assert_eq!(show(&t, "1234567.891"), "$1,234,567.89");
        assert_eq!(show(&t, "5.00"), "$5");
        assert_eq!(show(&t, "5.5"), "$5.50");
        assert_eq!(show(&t, "-5.5"), "-$5.50");
        assert_eq!(show(&t, "0.009"), "$0");
        assert_eq!(short_currency(&t.ess, &d("10.999")), "$10.99");
        assert_eq!(short_currency(&t.ess, &d("1000")), "$1000");
        assert_eq!(display_currency_exactly(&t.ess, &d("0.001")), "$0.001");
        t = TestServer::with_config("currency-symbol: '€'\ncurrency-symbol-suffix: true");
        assert_eq!(show(&t, "1234.5"), "1,234.50€");
        t = TestServer::with_config("currency-symbol: 'AB'");
        assert_eq!(show(&t, "1"), "$1");
        t = TestServer::with_config("currency-symbol-format-locale: de-DE");
        assert_eq!(show(&t, "1234.5"), "$1.234,50");
        t = TestServer::with_config("currency-format: '#0.0'");
        assert_eq!(show(&t, "1234.56"), "$1234.5");
    }

    #[test]
    fn amounts_parse_like_number_util() {
        let p = |s: &str| parse_amount(s).map(|d| d.to_string());
        assert_eq!(p("100"), Some("100".into()));
        assert_eq!(p("10.50"), Some("10.5".into()));
        assert_eq!(p("1,000"), Some("1000".into()));
        assert_eq!(p("1.5k"), Some("1500.0".into()));
        assert_eq!(p("2M"), Some("2000000".into()));
        assert_eq!(p("3b"), Some("3000000000".into()));
        assert_eq!(p("1t"), Some("1000000000000".into()));
        assert_eq!(p("10x"), None);
        assert_eq!(p("10%"), None);
        assert_eq!(p(""), None);
    }

    // ===== /balance =====================================================================================

    #[test]
    fn balance_shows_starting_balance_and_others() {
        let mut t = TestServer::with_config("starting-balance: 50");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        set_money(&mut t, bob, "12.5");
        assert_eq!(t.cmd(alice, "/balance"), ["Balance: $50"]);
        assert_eq!(t.cmd(alice, "/bal Bob"), ["Balance of Bob: $12.50"]);
        assert_eq!(t.cmd(alice, "/money"), ["Balance: $50"]);
        assert_eq!(t.console("balance Alice"), ["Balance of Alice: $50"]);
        assert!(t.console("balance")[0].contains("balance"), "console needs a player");
        // Without essentials.balance.others the argument is ignored.
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.balance]\n");
        assert_eq!(t.cmd(alice, "/bal Bob"), ["Balance: $50"]);
        assert_eq!(t.cmd(alice, "/bal Bob extra")[0], "Command Help: /bal");
    }

    #[test]
    fn balance_of_unknown_player() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        assert_eq!(t.cmd(alice, "/balance Nobody"), ["Error: Player not found."]);
    }

    // ===== /eco =========================================================================================

    #[test]
    fn eco_give_take_set_reset() {
        let mut t = TestServer::with_config("starting-balance: 25");
        let alice = t.join("Alice");
        assert_eq!(t.console("eco give Alice 100"), ["$100 added to Alice account. New balance: $125"]);
        assert_eq!(t.said(alice), ["$100 has been added to your account."]);
        assert_eq!(money(&mut t, alice), "125");
        assert_eq!(t.console("eco take Alice 30"), ["$30 taken from Alice account. New balance: $95"]);
        assert_eq!(t.said(alice), ["$30 has been taken from your account."]);
        assert_eq!(t.console("eco set Alice 5000"), ["You set Alice's balance to $5,000."]);
        assert_eq!(t.said(alice), ["Your balance was set to $5,000."]);
        assert_eq!(money(&mut t, alice), "5000");
        t.console("eco reset Alice");
        assert_eq!(money(&mut t, alice), "25");
        assert_eq!(t.userdata(alice)["money"], "25");
    }

    #[test]
    fn eco_respects_min_and_max_money() {
        let mut t = TestServer::with_config("min-money: 0\nmax-money: 1000");
        let alice = t.join("Alice");
        t.console("eco set Alice 70");
        assert_eq!(t.console("eco take Alice 200"), ["Error: The minimum balance a user can have is $0."]);
        assert_eq!(money(&mut t, alice), "70");
        t.console("eco set Alice 5000");
        assert_eq!(money(&mut t, alice), "1000");
        // A negative amount isn't a number for parseStringToBDecimal ("-" is an unknown modifier).
        assert!(t.console("eco set Alice -50")[0].contains("eco"));
        assert_eq!(money(&mut t, alice), "1000");
        t.console("eco set Alice 990");
        assert_eq!(t.console("eco give Alice 20"), ["Error: This transaction would exceed the balance limit for this account."]);
        assert_eq!(money(&mut t, alice), "990");
    }

    #[test]
    fn min_money_is_always_negative_or_zero() {
        let mut t = TestServer::with_config("min-money: 100");
        let alice = t.join("Alice");
        t.console("eco take Alice 50");
        assert_eq!(money(&mut t, alice), "-50");
        assert_eq!(t.console("eco take Alice 100"), ["Error: The minimum balance a user can have is -$100."]);
    }

    #[test]
    fn eco_exact_decimals_and_suffixes() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        for _ in 0..3 {
            t.console("eco give Alice 0.1");
        }
        assert_eq!(money(&mut t, alice), "0.3");
        t.console("eco set Alice 1.5k");
        assert_eq!(money(&mut t, alice), "1500.0");
    }

    #[test]
    fn eco_usage_and_targets() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        assert!(t.console("eco")[0].contains("eco"));
        assert!(t.console("eco foo Alice 1")[0].contains("eco"));
        assert!(t.console("eco give Alice")[0].contains("eco"));
        assert_eq!(t.console("eco give Nobody 5"), ["Error: Player not found."]);
        t.console("eco give * 10");
        assert_eq!((money(&mut t, alice), money(&mut t, bob)), ("10".into(), "10".into()));
        t.quit(bob);
        t.console("eco give ** 1");
        assert_eq!((money(&mut t, alice), money(&mut t, bob)), ("11".into(), "11".into()));
        // An offline player is found by name.
        t.console("eco give Bob 1");
        assert_eq!(money(&mut t, bob), "12");
        // Players need essentials.eco (op by default).
        assert_eq!(t.cmd(alice, "/eco give Alice 5"), ["You do not have access to that command."]);
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/eco give Alice 5"), ["$5 has been added to your account.", "$5 added to Alice account. New balance: $16"]);
    }

    // ===== /pay =========================================================================================

    #[test]
    fn pay_moves_money() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        set_money(&mut t, alice, "100");
        assert_eq!(t.cmd(alice, "/pay Bob 25"), ["$25 has been sent to Bob."]);
        assert_eq!(t.said(bob), ["$25 has been received from Alice."]);
        assert_eq!((money(&mut t, alice), money(&mut t, bob)), ("75".into(), "25".into()));
        assert_eq!(t.cmd(alice, "/pay Bob 0.5"), ["$0.50 has been sent to Bob."]);
        assert_eq!(money(&mut t, bob), "25.5");
    }

    #[test]
    fn pay_validates_amounts() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        t.join("Bob");
        set_money(&mut t, alice, "10");
        assert_eq!(t.cmd(alice, "/pay Bob -5"), ["Error: Amount to pay must be positive."]);
        assert!(t.cmd(alice, "/pay Bob abc")[0].contains("pay"));
        assert!(t.cmd(alice, "/pay Bob")[0].contains("pay"));
        assert_eq!(t.cmd(alice, "/pay Bob 0.0001"), ["Error: The minimum amount you can pay is $0.001."]);
        assert_eq!(t.cmd(alice, "/pay Bob 20"), ["Error: You do not have sufficient funds."]);
        assert_eq!(t.cmd(alice, "/pay Nobody 1"), ["Error: Player not found."]);
        assert_eq!(money(&mut t, alice), "10");
        let mut t = TestServer::with_config("minimum-pay-amount: 1");
        let alice = t.join("Alice");
        t.join("Bob");
        assert_eq!(t.cmd(alice, "/pay Bob 0.5"), ["Error: The minimum amount you can pay is $1."]);
    }

    #[test]
    fn pay_with_loan_permission_goes_negative() {
        let mut t = TestServer::with_config("min-money: -50");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.pay, essentials.eco.loan]\n");
        let alice = t.join("Alice");
        t.join("Bob");
        assert_eq!(t.cmd(alice, "/pay Bob 30"), ["$30 has been sent to Bob."]);
        assert_eq!(money(&mut t, alice), "-30");
        assert_eq!(t.cmd(alice, "/pay Bob 30"), ["Error: You do not have sufficient funds."]);
    }

    #[test]
    fn paytoggle_and_its_aliases() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        set_money(&mut t, alice, "100");
        t.op("Bob");
        assert_eq!(t.cmd(bob, "/paytoggle"), ["You are no longer accepting payments."]);
        assert_eq!(t.userdata(bob)["accepting-pay"], false);
        assert_eq!(t.cmd(alice, "/pay Bob 5"), ["Bob is not accepting payment."]);
        assert_eq!(t.cmd(bob, "/payon"), ["You are now accepting payments."]);
        assert_eq!(t.cmd(bob, "/payoff"), ["You are no longer accepting payments."]);
        assert_eq!(t.cmd(bob, "/paytoggle on"), ["You are now accepting payments."]);
        assert_eq!(t.cmd(bob, "/paytoggle Alice"), ["Disabled accepting payments for Alice."]);
        assert_eq!(t.said(alice), ["You are no longer accepting payments."]);
        assert_eq!(t.console("paytoggle Alice on"), ["Enabled accepting payments for Alice."]);
        assert_eq!(t.said(alice), ["You are now accepting payments."]);
    }

    #[test]
    fn pay_confirmation() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        t.op("Alice");
        set_money(&mut t, alice, "100");
        assert_eq!(t.cmd(alice, "/payconfirmtoggle"), ["You will now be prompted to confirm payments."]);
        assert_eq!(t.userdata(alice)["confirm-pay"], true);
        assert_eq!(t.cmd(alice, "/pay Bob 10"), ["To CONFIRM payment of $10, please repeat command: /pay Bob 10"]);
        assert_eq!(money(&mut t, bob), "unset");
        assert_eq!(t.cmd(alice, "/pay Bob 10"), ["$10 has been sent to Bob."]);
        assert_eq!(money(&mut t, bob), "10");
        // A different amount asks again.
        assert_eq!(t.cmd(alice, "/pay Bob 5").len(), 1);
        assert_eq!(t.cmd(alice, "/pay Bob 6"), ["To CONFIRM payment of $6, please repeat command: /pay Bob 6"]);
        assert_eq!(t.cmd(alice, "/payconfirmoff"), ["You will no longer be prompted to confirm payments."]);
        assert_eq!(t.cmd(alice, "/pay Bob 6"), ["$6 has been sent to Bob."]);
        assert_eq!(t.cmd(alice, "/payconfirmon"), ["You will now be prompted to confirm payments."]);
    }

    #[test]
    fn pay_confirmation_enabled_by_default() {
        let mut t = TestServer::with_config("default-enabled-confirm-commands: [pay]");
        let alice = t.join("Alice");
        t.join("Bob");
        set_money(&mut t, alice, "100");
        assert_eq!(t.cmd(alice, "/pay Bob 1"), ["To CONFIRM payment of $1, please repeat command: /pay Bob 1"]);
    }

    #[test]
    fn pay_offline_and_wildcards() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        let carol = t.join("Carol");
        set_money(&mut t, alice, "100");
        t.quit(bob);
        assert_eq!(t.cmd(alice, "/pay Bob 5"), ["You cannot pay offline users."]);
        assert_eq!(t.cmd(alice, "/pay ** 5"), ["You cannot pay offline users."]);
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/pay Bob 5"), ["$5 has been sent to Bob."]);
        assert_eq!(money(&mut t, bob), "5");
        let lines = t.cmd(alice, "/pay * 1");
        assert!(lines.contains(&"$1 has been sent to Carol.".to_string()), "{lines:?}");
        assert_eq!(money(&mut t, carol), "1");
    }

    #[test]
    fn pay_excludes_ignore_list() {
        let mut t = TestServer::with_config("pay-excludes-ignore-list: true");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        set_money(&mut t, alice, "100");
        t.ess.users.user(bob).set("ignore", json!([uuid_string(alice)]));
        assert_eq!(t.cmd(alice, "/pay Bob 5"), ["Bob is not accepting payment."]);
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        set_money(&mut t, alice, "100");
        t.ess.users.user(bob).set("ignore", json!([uuid_string(alice)]));
        assert_eq!(t.cmd(alice, "/pay Bob 5"), ["$5 has been sent to Bob."]);
    }

    #[test]
    fn pay_over_max_money_is_refunded() {
        let mut t = TestServer::with_config("max-money: 100");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        set_money(&mut t, alice, "50");
        set_money(&mut t, bob, "90");
        assert_eq!(t.cmd(alice, "/pay Bob 20"), ["This transaction would exceed the balance limit for this account."]);
        assert_eq!((money(&mut t, alice), money(&mut t, bob)), ("50".into(), "90".into()));
    }

    #[test]
    fn economy_log_writes_csv() {
        let mut t = TestServer::with_config("economy-log-enabled: true");
        let alice = t.join("Alice");
        t.join("Bob");
        set_money(&mut t, alice, "100");
        t.cmd(alice, "/pay Bob 25");
        let log = std::fs::read_to_string(t.ess.dir.join("trade.log")).expect("trade.log");
        let line = log.lines().find(|l| l.starts_with("Command,Pay,Player,\"")).expect("a pay line");
        assert!(line.contains("\",\"Alice\",25,money,$,\"Bob\",25,money,$,\"minecraft:overworld\",0,64,0,75,"), "{line}");
        // Balance updates are only logged with economy-log-update-enabled.
        assert!(!log.contains("Update,Set,API"));
        let mut t = TestServer::with_config("economy-log-update-enabled: true\neconomy-log-uuids: true");
        let alice = t.join("Alice");
        t.console("eco set Alice 5");
        let log = std::fs::read_to_string(t.ess.dir.join("trade.log")).expect("trade.log");
        let expected = format!("\",\"{}\",5,money,$,\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\"5,", uuid_string(alice));
        assert!(log.lines().any(|l| l.starts_with("Update,Set,API,\"") && l.contains(&expected)), "{log}");
    }

    // ===== /balancetop ==================================================================================

    fn baltop_server(config: &str) -> (TestServer, Uuid, Uuid, Uuid) {
        let mut t = TestServer::with_config(config);
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        let carol = t.join("Carol");
        set_money(&mut t, alice, "100");
        set_money(&mut t, carol, "300");
        (t, alice, bob, carol)
    }

    #[test]
    fn balancetop_lists_and_totals() {
        let (mut t, alice, _, _) = baltop_server("");
        let lines = t.cmd(alice, "/baltop");
        assert!(lines[0].starts_with("Top balances ("), "{lines:?}");
        assert_eq!(&lines[1..], [" ---- Balancetop -- Page 1/1 ----", "Server Total: $400", "1. Carol, $300", "2. Alice, $100", "3. Bob, $0"]);
    }

    #[test]
    fn balancetop_requirements_limit_and_exclusions() {
        let (mut t, alice, _, _) = baltop_server("show-zero-baltop: false");
        assert_eq!(&t.cmd(alice, "/baltop")[2..], ["Server Total: $400", "1. Carol, $300", "2. Alice, $100"]);
        let (mut t, alice, _, _) = baltop_server("baltop-requirements:\n  minimum-balance: 150\n  minimum-playtime: 0");
        assert_eq!(&t.cmd(alice, "/baltop")[2..], ["Server Total: $400", "1. Carol, $300"]);
        let (mut t, alice, _, _) = baltop_server("baltop-entry-limit: 1");
        assert_eq!(&t.cmd(alice, "/baltop")[2..], ["Server Total: $400", "1. Carol, $300"]);
        let (mut t, alice, _, _) = baltop_server("");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.balancetop]\nusers:\n  carol:\n    permissions: [essentials.balancetop.exclude]\n");
        assert_eq!(&t.cmd(alice, "/baltop")[2..], ["Server Total: $100", "1. Alice, $100", "2. Bob, $0"]);
        // Playtime requirement (online players' play_time statistic, in ticks).
        let (mut t, alice, _, carol) = baltop_server("baltop-requirements:\n  minimum-balance: 0\n  minimum-playtime: 60");
        t.host.p(carol).stats.insert("minecraft:custom/minecraft:play_time".into(), 20 * 61);
        assert_eq!(&t.cmd(alice, "/baltop")[2..], ["Server Total: $400", "1. Carol, $300"]);
    }

    #[test]
    fn balancetop_cache_and_force() {
        let (mut t, alice, bob, _) = baltop_server("");
        t.cmd(alice, "/baltop");
        set_money(&mut t, bob, "1000");
        assert_eq!(t.cmd(alice, "/baltop")[3], "1. Carol, $300", "cached for two minutes");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/baltop force")[3], "1. Bob, $1,000");
        set_money(&mut t, bob, "1");
        t.host.now += 2 * 60 * 1000 + 1;
        assert_eq!(t.cmd(alice, "/baltop")[3], "1. Carol, $300");
    }

    #[test]
    fn balancetop_pages() {
        let mut t = TestServer::new();
        let first = t.join("P00");
        for i in 1..12 {
            let p = t.join(&format!("P{i:02}"));
            set_money(&mut t, p, &format!("{}", i * 10));
        }
        let page1 = t.cmd(first, "/baltop");
        assert_eq!(page1[1], " ---- Balancetop -- Page 1/2 ----");
        assert_eq!(page1.len(), 1 + 1 + 9 + 1);
        assert_eq!(page1.last().unwrap(), "Type /balancetop 2 to read the next page.");
        let page2 = t.cmd(first, "/baltop 2");
        assert_eq!(page2[1], " ---- Balancetop -- Page 2/2 ----");
        assert_eq!(page2[2..], ["9. P03, $30", "10. P02, $20", "11. P01, $10", "12. P00, $0"]);
        assert_eq!(t.cmd(first, "/baltop 3")[1], "Unknown chapter.");
    }

    // ===== /worth, /sell, /setworth =========================================================================

    /// A server whose worth.yml starts empty, and without the default double/triple multipliers ops would get.
    fn shop_with(config: &str) -> TestServer {
        // `config` lines starting with two spaces add sell-multipliers.
        let t = TestServer::with_config(&format!("sell-multipliers:\n  double: 1.0\n  triple: 1.0\n{config}"));
        std::fs::write(t.ess.dir.join("worth.yml"), "worth: {}\n").unwrap();
        t
    }
    fn shop() -> (TestServer, Uuid) {
        let mut t = shop_with("");
        let alice = t.join("Alice");
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/setworth diamond 10"), ["Worth value set"]);
        assert_eq!(t.console("setworth iron_ingot 2.5"), ["Worth value set"]);
        (t, alice)
    }

    #[test]
    fn setworth_writes_worth_yml() {
        let (mut t, alice) = shop();
        let worth = t.ess.read_yaml("worth.yml");
        assert_eq!(worth["worth"]["diamond"].as_f64(), Some(10.0));
        assert_eq!(worth["worth"]["ironingot"].as_f64(), Some(2.5));
        give(&mut t, alice, 0, "gold_ingot", 1);
        assert_eq!(t.cmd(alice, "/setworth 7"), ["Worth value set"]);
        assert_eq!(t.ess.read_yaml("worth.yml")["worth"]["goldingot"].as_f64(), Some(7.0));
        assert!(t.cmd(alice, "/setworth")[0].contains("setworth"));
        assert!(t.console("setworth 5")[0].contains("setworth"));
        assert_eq!(t.cmd(alice, "/setworth nosuchitem 5"), ["Error: Unknown item name: nosuchitem."]);
    }

    #[test]
    fn worth_yml_lookup_forms() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        std::fs::write(t.ess.dir.join("worth.yml"), "worth:\n  stone:\n    '0': 3\n  dirt:\n    '*': 1.5\n  oaklog: 2.0\n  cobblestone: -1\n").unwrap();
        give(&mut t, alice, 0, "stone", 2);
        give(&mut t, alice, 1, "dirt", 2);
        give(&mut t, alice, 2, "oak_log", 1);
        assert_eq!(t.cmd(alice, "/worth stone"), ["Stack of stone worth $6 (2 item(s) at $3 each)"]);
        assert_eq!(t.cmd(alice, "/worth dirt"), ["Stack of dirt worth $3 (2 item(s) at $1.50 each)"]);
        assert_eq!(t.cmd(alice, "/worth oaklog"), ["Stack of oaklog worth $2 (1 item(s) at $2 each)"]);
        assert_eq!(t.cmd(alice, "/worth cobblestone"), ["Error: That item cannot be sold to the server."]);
    }

    #[test]
    fn worth_command_forms() {
        let (mut t, alice) = shop();
        give(&mut t, alice, 0, "diamond", 5);
        assert_eq!(t.cmd(alice, "/worth"), ["Stack of diamond worth $50 (5 item(s) at $10 each)"]);
        assert_eq!(t.cmd(alice, "/worth diamond 2"), ["Stack of diamond worth $20 (2 item(s) at $10 each)"]);
        assert_eq!(t.cmd(alice, "/price diamond"), ["Stack of diamond worth $50 (5 item(s) at $10 each)"]);
        assert_eq!(t.console("worth diamond 3"), ["Stack of diamond worth $30 (3 item(s) at $10 each)"]);
        assert_eq!(t.console("worth diamond"), ["Stack of diamond worth $10 (1 item(s) at $10 each)"]);
        give(&mut t, alice, 1, "iron_ingot", 4);
        assert_eq!(
            t.cmd(alice, "/worth all"),
            [
                "Stack of diamond worth $50 (5 item(s) at $10 each)",
                "Stack of ironingot worth $10 (4 item(s) at $2.50 each)",
                "The total worth of all sellable items and blocks is $60."
            ]
        );
        let mut sword = Item::new("diamond_sword", 1);
        sword.components = json!({"damage": 10});
        t.host.p(alice).inventory[0] = Some(sword);
        t.cmd(alice, "/setworth diamond_sword 100");
        assert_eq!(t.cmd(alice, "/worth hand"), ["Stack of diamondsword with metadata of 10 worth $100 (1 item(s) at $100 each)"]);
    }

    #[test]
    fn sell_hand_item_and_amounts() {
        let (mut t, alice) = shop();
        give(&mut t, alice, 0, "diamond", 5);
        assert_eq!(t.cmd(alice, "/sell diamond 2"), ["$20 has been added to your account.", "Sold for $20 (2 diamond at $10 each)."]);
        assert_eq!(count(&mut t, alice, "diamond"), 3);
        assert_eq!(t.cmd(alice, "/sell diamond -1"), ["$20 has been added to your account.", "Sold for $20 (2 diamond at $10 each)."]);
        assert_eq!(
            t.cmd(alice, "/sell diamond 10"),
            [
                "If you meant to sell all of your items of that type, use /sell itemname.",
                "/sell itemname -1 will sell all but one item, etc.",
                "Error: You do not have enough of that item to sell."
            ]
        );
        assert_eq!(t.cmd(alice, "/sell hand"), ["$10 has been added to your account.", "Sold for $10 (1 diamond at $10 each)."]);
        assert_eq!(count(&mut t, alice, "diamond"), 0);
        // worth.yml holds doubles (10.0) and the multiplier is 1.0: BigDecimal keeps those scales, as EssentialsX does.
        assert_eq!(money(&mut t, alice), "50.00");
        assert_eq!(t.cmd(alice, "/sell hand"), ["Error: You really tried to sell Air? Put an item in your hand."]);
        give(&mut t, alice, 0, "stone", 1);
        assert_eq!(t.cmd(alice, "/sell stone"), ["Error: That item cannot be sold to the server."]);
        assert!(t.cmd(alice, "/sell")[0].contains("sell"));
        assert!(t.host.said(Sender::Console).iter().any(|l| l.contains("Alice sold diamond for $20 (2 items at $10 each).")));
    }

    #[test]
    fn sell_bulk() {
        let (mut t, alice) = shop();
        give(&mut t, alice, 0, "diamond", 2);
        give(&mut t, alice, 1, "iron_ingot", 4);
        give(&mut t, alice, 2, "stone", 3);
        give(&mut t, alice, 3, "diamond", 1);
        assert_eq!(
            t.cmd(alice, "/sell all"),
            [
                "$30 has been added to your account.",
                "Sold for $30 (3 diamond at $10 each).",
                "$10 has been added to your account.",
                "Sold for $10 (4 iron_ingot at $2.50 each).",
                "Sold all items and blocks for a total worth of $40."
            ]
        );
        assert_eq!(count(&mut t, alice, "stone"), 3);
        assert_eq!(money(&mut t, alice), "40.00");
    }

    #[test]
    fn sell_permissions() {
        let (mut t, alice) = shop();
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.sell, essentials.setworth]\n");
        t.host.p(alice).op = 0;
        give(&mut t, alice, 0, "diamond", 2);
        assert_eq!(t.cmd(alice, "/sell hand"), ["Error: You do not have permission to hand sell."]);
        assert_eq!(t.cmd(alice, "/sell all"), ["Error: You do not have permission to bulk sell."]);
        assert_eq!(t.cmd(alice, "/sell inventory"), ["Error: You do not have permission to bulk sell."]);
        assert_eq!(t.cmd(alice, "/sell diamond").len(), 2);
    }

    #[test]
    fn sell_named_items() {
        let (mut t, alice) = shop();
        let mut named = Item::new("diamond", 1);
        named.components = json!({"custom_name": "Precious"});
        t.host.p(alice).inventory[0] = Some(named.clone());
        assert_eq!(t.cmd(alice, "/sell hand"), ["Error: You are not allowed to sell named items."]);
        give(&mut t, alice, 1, "diamond", 1);
        let lines = t.cmd(alice, "/sell all");
        assert_eq!(lines[2], "Error: You are not allowed to sell these named items: Precious", "{lines:?}");
        assert_eq!(count(&mut t, alice, "diamond"), 1);
        let mut t = shop_with("allow-selling-named-items: true");
        let alice = t.join("Alice");
        t.op("Alice");
        t.cmd(alice, "/setworth diamond 10");
        t.host.p(alice).inventory[0] = Some(named);
        assert_eq!(t.cmd(alice, "/sell hand"), ["$10 has been added to your account.", "Sold for $10 (1 diamond at $10 each)."]);
    }

    #[test]
    fn sell_multipliers_and_stacks() {
        let mut t = shop_with("  vip: 2.5\ntrade-in-stacks:\n  ironingot: true");
        let alice = t.join("Alice");
        t.op("Alice");
        t.cmd(alice, "/setworth diamond 10");
        t.cmd(alice, "/setworth iron_ingot 1");
        t.host.p(alice).op = 0;
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.sell, essentials.sell.hand]\n");
        give(&mut t, alice, 0, "diamond", 1);
        assert_eq!(t.cmd(alice, "/sell hand")[1], "Sold for $10 (1 diamond at $10 each).");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.sell, essentials.sell.hand]\nusers:\n  alice:\n    permissions: [essentials.sell.multiplier.vip]\n");
        give(&mut t, alice, 0, "diamond", 1);
        assert_eq!(t.cmd(alice, "/sell hand")[1], "Sold for $25 (1 diamond at $25 each).");
        give(&mut t, alice, 1, "iron_ingot", 64);
        give(&mut t, alice, 2, "iron_ingot", 10);
        assert_eq!(t.cmd(alice, "/sell iron_ingot 1"), ["Error: Item must be traded in stacks. A quantity of 2s would be two stacks, etc."]);
        assert_eq!(t.cmd(alice, "/sell iron_ingot 1s")[1], "Sold for $160 (64 iron_ingot at $2.50 each).");
        assert_eq!(count(&mut t, alice, "iron_ingot"), 10);
    }

    // ===== Command costs ==================================================================================

    #[test]
    fn command_costs_charge_players() {
        let mut t = TestServer::with_config("command-costs:\n  ping: 5");
        let alice = t.join("Alice");
        set_money(&mut t, alice, "10");
        assert_eq!(t.cmd(alice, "/ping"), ["$5 has been taken from your account.", "Pong!"]);
        assert_eq!(t.cmd(alice, "/pong"), ["$5 has been taken from your account.", "Pong!"]);
        assert_eq!(t.cmd(alice, "/ping"), ["Error: You do not have sufficient funds."]);
        assert_eq!(money(&mut t, alice), "0");
        assert_eq!(t.console("ping"), ["Pong!"]);
        t.op("Alice");
        assert_eq!(t.cmd(alice, "/ping"), ["Pong!"], "ops have essentials.nocommandcost.all");
    }

    #[test]
    fn command_cost_exemption_per_command() {
        let mut t = TestServer::with_config("command-costs:\n  ping: 5");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.ping, essentials.nocommandcost.ping]\n");
        let alice = t.join("Alice");
        assert_eq!(t.cmd(alice, "/ping"), ["Pong!"]);
    }

    #[test]
    fn chat_costs_for_other_areas() {
        let mut t = TestServer::with_config("command-costs:\n  chat-shout: 2\n  chat-question: 1");
        let alice = t.join("Alice");
        set_money(&mut t, alice, "3");
        assert_eq!(charge_command_cost(&mut t.ess, &mut t.host, alice, "chat-shout"), Ok(()));
        assert_eq!(t.said(alice), ["$2 has been taken from your account."]);
        assert_eq!(charge_command_cost(&mut t.ess, &mut t.host, alice, "chat-shout"), Err(CmdError::Tl("notEnoughMoney", vec![Arg::Parsed("$2".into())])));
        assert_eq!(charge_command_cost(&mut t.ess, &mut t.host, alice, "chat-question"), Ok(()));
        assert_eq!(charge_command_cost(&mut t.ess, &mut t.host, alice, "chat-other"), Ok(()));
        assert_eq!(money(&mut t, alice), "0");
    }

    // ===== Signs: creation, permissions, colours ==============================================================

    #[test]
    fn signs_need_enabled_signs() {
        let mut t = TestServer::new();
        let alice = t.join("Alice");
        t.op("Alice");
        let (r, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Balance]", "", "", ""]);
        assert_eq!((r, l[0].as_str()), (Allow::Yes, "[Balance]"), "no signs enabled: untouched");
        let mut t = TestServer::with_config("enabledSigns: [buy]");
        let alice = t.join("Alice");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Balance]", "", "", ""]);
        assert_eq!(l[0], "[balance]", "a disabled Essentials sign is only normalized");
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], [" [Buy] ", "1", "diamond", "10"]);
        assert_eq!(l, ["§1[Buy]", "1", "diamond", "$10"]);
    }

    #[test]
    fn sign_create_permission() {
        let mut t = signs_config("");
        let bob = t.join("Bob");
        let (r, l) = write_sign(&mut t, bob, [0, 64, 0], ["[Buy]", "1", "diamond", "10"]);
        assert_eq!((r, l[0].as_str()), (Allow::Yes, "[buy]"));
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.signs.buy.create]\n");
        let (_, l) = write_sign(&mut t, bob, [0, 64, 0], ["[Buy]", "1", "diamond", "10"]);
        assert_eq!(l[0], "§1[Buy]");
    }

    #[test]
    fn sign_create_errors_mark_the_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        let (r, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Buy]", "x", "diamond", "10"]);
        assert_eq!((r, l[0].as_str()), (Allow::Yes, "§4[Buy]"));
        assert_eq!(t.said(alice), ["Error: Invalid sign"]);
        write_sign(&mut t, alice, [0, 64, 0], ["[Buy]", "1", "nosuchitem", "10"]);
        assert_eq!(t.said(alice), ["Error: Unknown item name: nosuchitem."]);
        write_sign(&mut t, alice, [0, 64, 0], ["[Buy]", "0", "diamond", "10"]);
        assert_eq!(t.said(alice), ["Error: Quantities must be greater than 0."]);
        write_sign(&mut t, alice, [0, 64, 0], ["[Sell]", "1", "diamond", "lots"]);
        assert_eq!(t.said(alice), ["Error: Invalid charge."]);
    }

    #[test]
    fn colour_signs() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["&cHello", "&lbold", "", ""]);
        assert_eq!(l[..2], ["§cHello", "§lbold"]);
        let (_, l) = write_sign(&mut t, bob, [0, 64, 0], ["&cHello", "plain & text", "", ""]);
        assert_eq!(l[..2], ["Hello", "plain & text"]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.signs.color, -essentials.signs.red]\n");
        let (_, l) = write_sign(&mut t, bob, [0, 64, 0], ["&aok &cno", "", "", ""]);
        assert_eq!(l[0], "§aok no");
    }

    // ===== Signs: interaction ==============================================================================

    #[test]
    fn balance_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        set_money(&mut t, alice, "42");
        write_sign(&mut t, alice, [0, 64, 0], ["[Balance]", "", "", ""]);
        assert_eq!(sign(&mut t, [0, 64, 0])[0], "§1[Balance]");
        assert_eq!(click(&mut t, alice, [0, 64, 0]), (Allow::No, vec!["Balance: $42".to_string()]));
        // Default players may use every sign (essentials.signs.use.*).
        let bob = t.join("Bob");
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["Balance: $0"]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [-essentials.signs.use.balance]\n");
        assert_eq!(click(&mut t, bob, [0, 64, 0]), (Allow::No, vec![]));
    }

    #[test]
    fn sign_use_per_second() {
        let mut t = signs_config("sign-use-per-second: 2");
        let alice = t.join("Alice");
        t.op("Alice");
        write_sign(&mut t, alice, [0, 64, 0], ["[Balance]", "", "", ""]);
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1.len(), 1);
        let state = t.host.block(W, [0, 64, 0]).unwrap();
        t.host.now += 100;
        t.ess.on_interact(&mut t.host, alice, Some((W, [0, 64, 0], &state)), None);
        assert!(t.said(alice).is_empty(), "throttled");
        t.host.now += 500;
        t.ess.on_interact(&mut t.host, alice, Some((W, [0, 64, 0], &state)), None);
        assert_eq!(t.said(alice).len(), 1);
    }

    #[test]
    fn buy_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        write_sign(&mut t, alice, [0, 64, 0], ["[Buy]", "2", "diamond", "10"]);
        let bob = t.join("Bob");
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["Error: You do not have sufficient funds."]);
        set_money(&mut t, bob, "25");
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["$10 has been taken from your account."]);
        assert_eq!(count(&mut t, bob, "diamond"), 2);
        assert_eq!(money(&mut t, bob), "15");
        for slot in 0..36 {
            give(&mut t, bob, slot, "stone", 64);
        }
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["Error: Your inventory is full."]);
        assert_eq!(money(&mut t, bob), "15");
    }

    #[test]
    fn buy_sign_bulk_while_sneaking() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        write_sign(&mut t, alice, [0, 64, 0], ["[Buy]", "1", "diamond", "4"]);
        let bob = t.join("Bob");
        set_money(&mut t, bob, "100");
        give(&mut t, bob, 0, "diamond", 5);
        t.host.economy_sneaking.insert(bob);
        click(&mut t, bob, [0, 64, 0]);
        assert_eq!(count(&mut t, bob, "diamond"), 10);
        assert_eq!(money(&mut t, bob), "80");
        let mut t = signs_config("allow-bulk-buy-sell: false");
        let alice = t.join("Alice");
        t.op("Alice");
        write_sign(&mut t, alice, [0, 64, 0], ["[Buy]", "1", "diamond", "4"]);
        let bob = t.join("Bob");
        set_money(&mut t, bob, "100");
        give(&mut t, bob, 0, "diamond", 5);
        t.host.economy_sneaking.insert(bob);
        click(&mut t, bob, [0, 64, 0]);
        assert_eq!(money(&mut t, bob), "96");
    }

    #[test]
    fn sell_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Sell]", "2", "diamond", "$5"]);
        assert_eq!(l, ["§1[Sell]", "2", "diamond", "$5"]);
        let bob = t.join("Bob");
        give(&mut t, bob, 3, "diamond", 3);
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["$5 has been added to your account."]);
        assert_eq!(count(&mut t, bob, "diamond"), 1);
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["Error: You do not have 2x diamond."]);
        assert_eq!(money(&mut t, bob), "5");
    }

    #[test]
    fn exp_trades_on_signs() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Buy]", "30", "xp", "1"]);
        assert_eq!(l, ["§1[Buy]", "30", "exp", "$1"]);
        let bob = t.join("Bob");
        set_money(&mut t, bob, "1");
        click(&mut t, bob, [0, 64, 0]);
        assert_eq!(t.host.p(bob).exp, 30);
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], ["[Heal]", "10 exp", "", ""]);
        assert_eq!(l[1], "10 exp");
        t.host.p(bob).health = 5.0;
        assert_eq!(click(&mut t, bob, [1, 64, 0]).1, ["You have been healed."]);
        assert_eq!(t.host.p(bob).exp, 20);
        t.host.p(bob).exp = 3;
        assert_eq!(click(&mut t, bob, [1, 64, 0]).1, ["Error: You do not have enough experience."]);
    }

    #[test]
    fn item_and_times_costs() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        write_sign(&mut t, alice, [0, 64, 0], ["[Heal]", "1 diamond", "", ""]);
        let bob = t.join("Bob");
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["Error: You do not have 1x diamond."]);
        give(&mut t, bob, 0, "diamond", 2);
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["You have been healed."]);
        assert_eq!(count(&mut t, bob, "diamond"), 1);
        write_sign(&mut t, alice, [1, 64, 0], ["[Heal]", "2 times", "", ""]);
        assert_eq!(sign(&mut t, [1, 64, 0])[1], "2 times");
        click(&mut t, bob, [1, 64, 0]);
        assert_eq!(sign(&mut t, [1, 64, 0])[1], "1 times");
        click(&mut t, bob, [1, 64, 0]);
        assert_eq!(sign(&mut t, [1, 64, 0])[1], "0 times");
        assert_eq!(click(&mut t, bob, [1, 64, 0]).1, ["Error: Quantities must be greater than 0."]);
    }

    #[test]
    fn free_and_disposal_signs() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        write_sign(&mut t, alice, [0, 64, 0], ["[Free]", "diamond", "", ""]);
        click(&mut t, alice, [0, 64, 0]);
        let open = t.host.p(alice).open.clone().expect("a container");
        assert_eq!((open.rows, open.title.as_str()), (4, "DIAMOND"));
        assert_eq!(open.items.len(), 36);
        assert!(open.items.iter().all(|i| i.as_ref().is_some_and(|i| i.id == "minecraft:diamond" && i.count == 64)));
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], ["[Free]", "nosuchitem", "", ""]);
        assert_eq!((l[0].as_str(), l[1].as_str()), ("§4[Free]", "§c<item>"));
        write_sign(&mut t, alice, [2, 64, 0], ["[Disposal]", "", "", ""]);
        click(&mut t, alice, [2, 64, 0]);
        let open = t.host.p(alice).open.clone().expect("a container");
        assert_eq!((open.rows, open.title.as_str()), (4, "Disposal"));
        assert!(open.items.iter().all(Option::is_none));
        write_sign(&mut t, alice, [3, 64, 0], ["[Disposal]", "Trash", "Can", ""]);
        click(&mut t, alice, [3, 64, 0]);
        assert_eq!(t.host.p(alice).open.as_ref().unwrap().title, "Trash Can");
        // Without essentials.signs.disposal.name the custom lines are cleared.
        let bob = t.join("Bob");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.signs.create.disposal, essentials.signs.use.*]\n");
        let (_, l) = write_sign(&mut t, bob, [4, 64, 0], ["[Disposal]", "Trash", "", ""]);
        assert_eq!(l, ["§1[Disposal]", "", "", ""]);
    }

    #[test]
    fn station_signs() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        for (i, (name, station)) in [
            ("[Anvil]", Station::Anvil),
            ("[Cartography]", Station::Cartography),
            ("[Grindstone]", Station::Grindstone),
            ("[Loom]", Station::Loom),
            ("[Smithing]", Station::Smithing),
            ("[Workbench]", Station::Crafting),
        ]
        .into_iter()
        .enumerate()
        {
            let pos = [i as i32, 64, 0];
            let (_, l) = write_sign(&mut t, alice, pos, [name, "", "", ""]);
            assert_eq!(l[0], format!("§1{name}"));
            click(&mut t, alice, pos);
            assert_eq!(t.host.p(alice).station, Some(station));
        }
    }

    #[test]
    fn heal_and_gamemode_signs() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Heal]", "5", "", ""]);
        assert_eq!(l[1], "$5");
        set_money(&mut t, alice, "10");
        t.host.p(alice).health = 3.0;
        t.host.p(alice).food = 2;
        t.host.p(alice).fire = 40;
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["You have been healed.", "$5 has been taken from your account."]);
        assert_eq!((t.host.p(alice).health, t.host.p(alice).food, t.host.p(alice).fire), (20.0, 20, 0));
        // A dead player can't use signs at all (EssentialsSign checks isDead before the sign's own healDead).
        t.host.p(alice).health = 0.0;
        assert_eq!(click(&mut t, alice, [0, 64, 0]), (Allow::No, vec![]));
        t.host.p(alice).health = 20.0;
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], ["[GameMode]", "", "", ""]);
        assert_eq!(l[1], "Survival");
        write_sign(&mut t, alice, [2, 64, 0], ["[GameMode]", "creative", "", ""]);
        assert_eq!(click(&mut t, alice, [2, 64, 0]).1, ["Set game mode creative for Alice."]);
        assert_eq!(t.host.p(alice).mode, GameMode::Creative);
        write_sign(&mut t, alice, [3, 64, 0], ["[GameMode]", "bogus", "", ""]);
        assert_eq!(click(&mut t, alice, [3, 64, 0]).1, ["Error: Line 2 on sign is invalid."]);
    }

    #[test]
    fn time_and_weather_signs() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Time]", "day", "", ""]);
        assert_eq!(l[1], "§2Day");
        t.host.time.insert(W.into(), 30_000);
        click(&mut t, alice, [0, 64, 0]);
        assert_eq!(t.host.time[W], 48_000);
        write_sign(&mut t, alice, [1, 64, 0], ["[Time]", "night", "", ""]);
        click(&mut t, alice, [1, 64, 0]);
        assert_eq!(t.host.time[W], 48_000 + 37_700);
        let (_, l) = write_sign(&mut t, alice, [2, 64, 0], ["[Time]", "noon", "", ""]);
        assert_eq!(l[0], "§4[Time]");
        assert_eq!(t.said(alice), ["Error: /time only supports day/night."]);
        write_sign(&mut t, alice, [3, 64, 0], ["[Time]", "", "", ""]);
        t.host.time.insert(W.into(), 6000);
        assert_eq!(click(&mut t, alice, [3, 64, 0]).1, ["The current time is 12:00 or 12:00 PM or 6000 ticks."]);
        let (_, l) = write_sign(&mut t, alice, [4, 64, 0], ["[Weather]", "storm", "", ""]);
        assert_eq!(l[1], "§2Storm");
        click(&mut t, alice, [4, 64, 0]);
        assert_eq!(t.host.weather[W].0, true);
        write_sign(&mut t, alice, [5, 64, 0], ["[Weather]", "", "", ""]);
        assert_eq!(click(&mut t, alice, [5, 64, 0]).1, ["Weather: stormy."]);
        write_sign(&mut t, alice, [6, 64, 0], ["[Weather]", "sun", "", ""]);
        click(&mut t, alice, [6, 64, 0]);
        assert_eq!(click(&mut t, alice, [5, 64, 0]).1, ["Weather: sunny."]);
        let (_, l) = write_sign(&mut t, alice, [7, 64, 0], ["[Weather]", "snow", "", ""]);
        assert_eq!((l[0].as_str(), l[1].as_str()), ("§4[Weather]", "§c<sun|storm>"));
        assert_eq!(t.said(alice), ["Error: /weather only supports sun/storm."]);
    }

    #[test]
    fn enchant_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Enchant]", "diamond_sword", "sharpness:9", ""]);
        assert_eq!(l[..3], ["§1[Enchant]", "diamond_sword", "sharpness:5"], "capped at the max level");
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], ["[Enchant]", "any", "nonsense", ""]);
        assert_eq!((l[0].as_str(), l[2].as_str()), ("§4[Enchant]", "§c<enchant>"));
        assert_eq!(t.said(alice), ["Error: Enchantment not found!"]);
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["Error: You do not have 1x diamond_sword."]);
        give(&mut t, alice, 0, "stone_sword", 1);
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["Error: You do not have 1x diamond sword."]);
        give(&mut t, alice, 0, "diamond_sword", 1);
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["The enchantment sharpness has been applied to your item in hand."]);
        let held = t.host.p(alice).inventory[0].clone().unwrap();
        assert_eq!(held.components["enchantments"]["minecraft:sharpness"], 5);
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["Error: You do not have 1x diamond_sword."], "already has it");
        write_sign(&mut t, alice, [2, 64, 0], ["[Enchant]", "*", "sharpness:0", ""]);
        assert_eq!(click(&mut t, alice, [2, 64, 0]).1, ["The enchantment sharpness has been removed from your item in hand."]);
    }

    #[test]
    fn repair_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        t.op("Alice");
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Repair]", "", "", ""]);
        assert_eq!(l[1], "Hand");
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], ["[Repair]", "legs", "", ""]);
        assert_eq!((l[0].as_str(), l[1].as_str()), ("§4[Repair]", "§c<hand|all>"));
        assert_eq!(t.said(alice), ["Error: Line 2 on sign is invalid."]);
        let mut sword = Item::new("diamond_sword", 1);
        sword.components = json!({"damage": 100});
        t.host.p(alice).inventory[0] = Some(sword.clone());
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["You have successfully repaired your: diamond sword."]);
        assert!(t.host.p(alice).inventory[0].as_ref().unwrap().components.get("damage").is_none_or(|d| d == 0));
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["Error: This item cannot be repaired."]);
        write_sign(&mut t, alice, [2, 64, 0], ["[Repair]", "all", "", ""]);
        t.host.p(alice).inventory[3] = Some(sword.clone());
        let mut bow = Item::new("bow", 1);
        bow.components = json!({"damage": 3});
        t.host.p(alice).inventory[5] = Some(bow);
        assert_eq!(click(&mut t, alice, [2, 64, 0]).1, ["You have successfully repaired your: diamond sword, bow."]);
        assert_eq!(click(&mut t, alice, [2, 64, 0]).1, ["Error: There were no items that needed repairing."]);
    }

    // Commands of other areas the signs dispatch, recorded.
    #[derive(Default)]
    struct Dispatched(Vec<(Sender, String)>);
    fn record(ctx: &mut Ctx, args: &[String]) -> CmdResult {
        let line = format!("{} {}", ctx.name, args.join(" "));
        let sender = ctx.sender;
        ctx.ess.state::<Dispatched>().0.push((sender, line.trim().to_string()));
        Ok(())
    }
    fn fake_commands(t: &mut TestServer) {
        for name in ["warp", "kit", "mail", "info", "tpr", "spawnmob"] {
            t.ess.register(Command { name, run: record, tab: None, usages: &[] });
        }
    }

    #[test]
    fn signs_of_other_areas_dispatch_commands() {
        let mut t = signs_config("");
        fake_commands(&mut t);
        let alice = t.join("Alice");
        t.op("Alice");
        std::fs::create_dir_all(t.ess.dir.join("warps")).unwrap();
        std::fs::write(t.ess.dir.join("warps/market.yml"), "name: market\nworld: world\nx: 1\ny: 2\nz: 3\nyaw: 0\npitch: 0\n").unwrap();
        std::fs::write(t.ess.dir.join("kits.yml"), "kits:\n  tools:\n    delay: 10\n    items:\n    - stone_sword 1\n").unwrap();
        let (_, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Warp]", "market", "everyone", ""]);
        assert_eq!(l[..3], ["§1[Warp]", "market", "§2Everyone"]);
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], ["[Warp]", "nowhere", "", ""]);
        assert_eq!(l[0], "§4[Warp]");
        assert_eq!(t.said(alice), ["Error: That warp does not exist."]);
        let (_, l) = write_sign(&mut t, alice, [2, 64, 0], ["[Kit]", "Tools", "", ""]);
        assert_eq!(l[0], "§1[Kit]");
        let (_, l) = write_sign(&mut t, alice, [3, 64, 0], ["[Kit]", "", "", ""]);
        assert_eq!((l[0].as_str(), l[1].as_str()), ("§4[Kit]", "§dKit name!"));
        write_sign(&mut t, alice, [4, 64, 0], ["[Mail]", "", "", ""]);
        write_sign(&mut t, alice, [5, 64, 0], ["[Info]", "rules", "2", ""]);
        write_sign(&mut t, alice, [6, 64, 0], ["[RandomTeleport]", "", "", ""]);
        let (_, l) = write_sign(&mut t, alice, [7, 64, 0], ["[Spawnmob]", "2", "zombie", ""]);
        assert_eq!(l[0], "§1[Spawnmob]");
        let bob = t.join("Bob");
        for x in [0, 2, 4, 5, 6, 7] {
            click(&mut t, bob, [x, 64, 0]);
        }
        let got = std::mem::take(&mut t.ess.state::<Dispatched>().0);
        assert_eq!(
            got,
            [
                (Sender::Console, "warp market Bob".to_string()),
                (Sender::Player(bob), "kit tools".to_string()),
                (Sender::Player(bob), "mail read".to_string()),
                (Sender::Player(bob), "info rules 2".to_string()),
                (Sender::Console, "tpr default Bob".to_string()),
                (Sender::Console, "spawnmob zombie 2 Bob".to_string()),
            ]
        );
        // A kit sign for a group the player is not in.
        write_sign(&mut t, alice, [8, 64, 0], ["[Kit]", "tools", "vip", ""]);
        assert_eq!(click(&mut t, bob, [8, 64, 0]).1, ["Error: You do not have access to this kit."]);
        // A warp sign for a group.
        write_sign(&mut t, alice, [9, 64, 0], ["[Warp]", "market", "vip", ""]);
        assert_eq!(click(&mut t, bob, [9, 64, 0]).1, ["Error: You do not have permission to use that warp."]);
    }

    #[test]
    fn trade_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.signs.create.trade, essentials.signs.break.trade, essentials.signs.use.*]\n");
        give(&mut t, alice, 0, "diamond", 5);
        let (r, l) = write_sign(&mut t, alice, [0, 64, 0], ["[Trade]", "10", "1 diamond 5", ""]);
        assert_eq!((r, l.clone()), (Allow::Yes, ["§1[Trade]", "$10:0", "1 diamond:5", "§8Alice"].map(String::from)));
        assert_eq!(count(&mut t, alice, "diamond"), 0, "the stock is taken from the owner");
        // The same type on both sides is refused.
        let (_, l) = write_sign(&mut t, alice, [1, 64, 0], ["[Trade]", "5", "$5 10", ""]);
        assert_eq!(l[0], "§4[Trade]");
        assert_eq!(t.said(alice), ["Error: You cannot trade for the same item type."]);
        set_money(&mut t, bob, "25");
        assert_eq!(click(&mut t, bob, [0, 64, 0]).1, ["$10 has been taken from your account."]);
        assert_eq!(count(&mut t, bob, "diamond"), 1);
        assert_eq!(sign(&mut t, [0, 64, 0])[1..3], ["$10:10", "1 diamond:4"]);
        // The owner collects the money.
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["$10 has been added to your account."]);
        assert_eq!(sign(&mut t, [0, 64, 0])[1], "$10:0");
        assert_eq!(money(&mut t, alice), "10");
        assert_eq!(click(&mut t, alice, [0, 64, 0]).1, ["Error: There is nothing to collect from this trade sign."]);
        // The owner restocks with the item in hand.
        give(&mut t, alice, 0, "diamond", 3);
        click(&mut t, alice, [0, 64, 0]);
        assert_eq!(sign(&mut t, [0, 64, 0])[2], "1 diamond:5");
        assert_eq!(count(&mut t, alice, "diamond"), 2);
        // Others may not break it; the owner gets the stock back.
        assert_eq!(break_at(&mut t, bob, [0, 64, 0]).0, Allow::No);
        assert_eq!(break_at(&mut t, alice, [0, 64, 0]).0, Allow::Yes);
        assert_eq!(count(&mut t, alice, "diamond"), 7);
        // Nothing left for a buyer.
        write_sign(&mut t, alice, [2, 64, 0], ["[Trade]", "10", "1 diamond 1", ""]);
        click(&mut t, bob, [2, 64, 0]);
        assert_eq!(click(&mut t, bob, [2, 64, 0]).1, ["Error: The trade sign has nothing available for you."]);
    }

    #[test]
    fn sign_break_permission_and_support() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        t.op("Alice");
        write_sign(&mut t, alice, [0, 64, 0], ["[Balance]", "", "", ""]);
        assert_eq!(break_at(&mut t, bob, [0, 64, 0]), (Allow::No, vec![]));
        // The block holding a standing sign up can't be broken, nor the one behind a wall sign.
        assert_eq!(break_at(&mut t, alice, [0, 63, 0]).0, Allow::No);
        t.host.blocks.insert((W.into(), [5, 64, 1]), "minecraft:oak_wall_sign[facing=south,waterlogged=false]".into());
        t.host.signs.insert((W.into(), [5, 64, 1]), ["§1[Balance]", "", "", ""].map(String::from));
        t.host.blocks.insert((W.into(), [5, 64, 0]), "minecraft:stone".into());
        assert_eq!(break_at(&mut t, alice, [5, 64, 0]).0, Allow::No);
        assert_eq!(break_at(&mut t, alice, [0, 64, 0]).0, Allow::Yes);
        assert_eq!(break_at(&mut t, bob, [9, 63, 9]).0, Allow::Yes);
        // Explosions spare Essentials signs.
        assert_eq!(t.ess.on_world(&mut t.host, "explode", W, [0, 64, 0]), Allow::No);
        assert_eq!(t.ess.on_world(&mut t.host, "explode", W, [9, 64, 9]), Allow::Yes);
    }

    #[test]
    fn protection_sign() {
        let mut t = signs_config("");
        let alice = t.join("Alice");
        let bob = t.join("Bob");
        let chest = [0, 64, 0];
        t.host.blocks.insert((W.into(), chest), "minecraft:chest[facing=north,type=single,waterlogged=false]".into());
        let (r, l) = write_sign(&mut t, alice, [1, 64, 0], ["[Protection]", "", "", ""]);
        assert_eq!((r, l), (Allow::Yes, ["§1[Protection]", "", "", "§1Alice"].map(String::from)));
        let (r, _) = write_sign(&mut t, alice, [9, 64, 9], ["[Protection]", "", "", ""]);
        assert_eq!(r, Allow::No);
        assert_eq!(t.said(alice), ["You are not allowed to create sign here."]);
        t.host.blocks.remove(&(W.to_string(), [9, 64, 9]));
        assert_eq!(click(&mut t, bob, chest), (Allow::No, vec!["You do not have permission to access that chest.".to_string()]));
        assert_eq!(break_at(&mut t, bob, chest), (Allow::No, vec!["You do not have permission to destroy that chest.".to_string()]));
        assert_eq!(click(&mut t, alice, chest).0, Allow::Yes);
        assert_eq!(t.ess.on_block_place(&mut t.host, bob, W, [0, 64, 1], "minecraft:chest[facing=north,type=single,waterlogged=false]"), Allow::No);
        assert_eq!(t.ess.on_block_place(&mut t.host, bob, W, [0, 64, 1], "minecraft:hopper[enabled=true,facing=down]"), Allow::Yes);
        assert_eq!(t.said(bob), ["You do not have permission to place a block near that sign."]);
        assert_eq!(break_at(&mut t, bob, [1, 64, 0]).0, Allow::No);
        // A name on line 2 or 3 lets that player in.
        write_sign(&mut t, alice, [1, 64, 0], ["[Protection]", "Bob", "", ""]);
        assert_eq!(click(&mut t, bob, chest).0, Allow::Yes);
    }
}
