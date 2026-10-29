// horizon-essentials area `world`, module `ess_geoip` (pure logic, standalone-tested): EssentialsX GeoIP: MaxMind .mmdb reader, join message, whois line.
//
// EssentialsGeoIPPlayerListener: on join, players with `essentials.geoip.show` read where a joining player (without
// `essentials.geoip.hide`) comes from; /whois shows the same location. The database is MaxMind's GeoIP2/GeoLite2
// .mmdb, read here by our own decoder (no MaxMind library). Off until `geoip/GeoIP2-Country.mmdb` (or
// `GeoIP2-City.mmdb` with database.show-cities) is in Essentials' data folder: the download needs a binary
// .tar.gz, which the Host's text-only HTTP answer can't carry, so the admin places the file (see `on_http`).

use crate::game::ess_core::*;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;

/// HTTP request ids this module starts carry this tag in their high bits (the world area routes answers by it).
pub const HTTP_ID: u64 = 0x4745_0000_0000_0000;

pub fn register(ess: &mut Ess) {
    ess.listeners.join.push(on_join);
}

// ===== State =======================================================================================

#[derive(Default)]
pub struct GeoState {
    /// The configuration was read (EssentialsGeoIPPlayerListener.reloadConfig ran).
    pub loaded: bool,
    /// A database is there or a license key may fetch one; otherwise the module is off (silent).
    pub enabled: bool,
    pub reader: Option<Mmdb>,
    pub config: Value,
    /// User.getGeoLocation (in memory, as EssentialsX keeps it).
    pub locations: HashMap<Uuid, String>,
    next_id: u64,
}

fn config_bool(cfg: &Value, path: &str, default: bool) -> bool {
    crate::game::ess_yaml::get(cfg, path).and_then(Value::as_bool).unwrap_or(default)
}
fn config_str(cfg: &Value, path: &str) -> Option<String> {
    crate::game::ess_yaml::get(cfg, path).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn log(ess: &Ess, host: &mut dyn Host, line: &str) {
    host.send(Sender::Console, &ess.render(&crate::game::ess_text::escape_tags(line)));
}

/// The plain text of a message key (tlLiteral + miniToLegacy for the log).
fn literal(ess: &Ess, key: &str, args: &[Arg]) -> String {
    crate::game::ess_text::plain(&ess.render(&ess.tl(key, args)))
}

pub fn database_file(ess: &mut Ess) -> PathBuf {
    let cities = config_bool(&ess.state::<GeoState>().config, "database.show-cities", false);
    ess.dir.join("geoip").join(if cities { "GeoIP2-City.mmdb" } else { "GeoIP2-Country.mmdb" })
}

/// EssentialsGeoIPPlayerListener.reloadConfig: reads geoip/config.yml (migrating pre-GeoIP2 files), downloads or
/// updates the database when configured, then opens it.
pub fn reload(ess: &mut Ess, host: &mut dyn Host) {
    let defaults = crate::game::ess_defaults::file("geoip/config.yml")
        .and_then(|t| crate::game::ess_yaml::parse(t).ok())
        .unwrap_or(json!({}));
    let file = ess.read_yaml("geoip/config.yml");
    let mut config = defaults;
    if file.is_object() {
        merge(&mut config, file.clone());
    }
    if file.is_object() && crate::game::ess_yaml::get(&file, "enable-locale").is_none() {
        let mut migrated = file;
        for (k, v) in [
            ("database.download-url", json!("https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key={LICENSEKEY}&suffix=tar.gz")),
            ("database.download-url-city", json!("https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key={LICENSEKEY}&suffix=tar.gz")),
            ("database.license-key", json!("")),
            ("database.update.enable", json!(true)),
            ("database.update.by-every-x-days", json!(30)),
            ("enable-locale", json!(true)),
        ] {
            crate::game::ess_yaml::set(&mut migrated, k, v.clone());
            crate::game::ess_yaml::set(&mut config, k, v);
        }
        ess.write_yaml("geoip/config.yml", &migrated);
        let _ = std::fs::remove_file(ess.dir.join("geoip/GeoIP.dat"));
        let _ = std::fs::remove_file(ess.dir.join("geoip/GeoIP-City.dat"));
    }
    {
        let st = ess.state::<GeoState>();
        st.config = config.clone();
        st.loaded = true;
        st.reader = None;
    }
    let path = database_file(ess);
    let key = config_str(&config, "database.license-key").unwrap_or_default();
    // Off by default: without a database file or a license key to fetch one, GeoIP stays silent.
    let enabled = path.exists() || !key.is_empty();
    ess.state::<GeoState>().enabled = enabled;
    if !enabled {
        return;
    }
    if !path.exists() {
        if config_bool(&config, "database.download-if-missing", true) {
            download(ess, host);
        } else {
            let msg = literal(ess, "cantFindGeoIpDB", &[]);
            log(ess, host, &msg);
        }
        return;
    }
    if config_bool(&config, "database.update.enable", true) {
        let age_days = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| std::time::SystemTime::now().duration_since(t).ok())
            .map_or(0, |d| d.as_secs() / 86_400);
        let every = crate::game::ess_yaml::get(&config, "database.update.by-every-x-days").and_then(Value::as_u64).unwrap_or(30);
        if age_days > every {
            download(ess, host);
        }
    }
    match std::fs::read(&path).ok().and_then(Mmdb::new) {
        Some(mut reader) => {
            // enable-locale: Essentials' locale first, then English (MaxMind's DatabaseReader locales).
            let mut locales = Vec::new();
            if config_bool(&config, "enable-locale", false) {
                let mut locale = ess.config_str("locale").unwrap_or_default().replace('_', "-");
                if locale.eq_ignore_ascii_case("zh") {
                    locale = "zh-CN".into();
                }
                if !locale.is_empty() {
                    locales.push(locale);
                }
            }
            locales.push("en".into());
            reader.locales = locales;
            ess.state::<GeoState>().reader = Some(reader);
        }
        None => {
            let msg = literal(ess, "cantReadGeoIpDB", &[]);
            log(ess, host, &msg);
        }
    }
}

/// downloadDatabase: the URL with the license key. Only the request is ours to make: see `on_http`.
fn download(ess: &mut Ess, host: &mut dyn Host) {
    let config = ess.state::<GeoState>().config.clone();
    let cities = config_bool(&config, "database.show-cities", false);
    let url = config_str(&config, if cities { "database.download-url-city" } else { "database.download-url" }).unwrap_or_default();
    if url.is_empty() {
        let msg = literal(ess, "geoIpUrlEmpty", &[]);
        log(ess, host, &msg);
        return;
    }
    let key = config_str(&config, "database.license-key").unwrap_or_default();
    if key.is_empty() {
        let msg = literal(ess, "geoIpLicenseMissing", &[]);
        log(ess, host, &msg);
        return;
    }
    let url = url.replace("{LICENSEKEY}", &key);
    if !url.starts_with("http://") && !url.starts_with("https://") {
        let msg = literal(ess, "geoIpUrlInvalid", &[]);
        log(ess, host, &msg);
        return;
    }
    let msg = literal(ess, "downloadingGeoIp", &[]);
    log(ess, host, &msg);
    let id = {
        let st = ess.state::<GeoState>();
        st.next_id += 1;
        HTTP_ID | st.next_id
    };
    host.http_request(id, "GET", &url, &[], None);
}

/// An HTTP answer for one of our ids (HTTP_ID tag); returns false for other modules' ids. The database comes as
/// a binary .tar.gz that the Host's text answer can't hold, so the admin is told where to place the .mmdb.
pub fn on_http(ess: &mut Ess, host: &mut dyn Host, id: u64, status: u16, _body: &str) -> bool {
    if id & 0xFFFF_0000_0000_0000 != HTTP_ID {
        return false;
    }
    if status != 200 {
        let msg = literal(ess, "connectionFailed", &[]);
        log(ess, host, &msg);
    }
    let path = database_file(ess);
    let msg = literal(ess, "cantFindGeoIpDB", &[]);
    log(ess, host, &format!("{msg} Download it from MaxMind and place it at {}", path.display()));
    true
}

// ===== Join ======================================================================================

fn on_join(ess: &mut Ess, host: &mut dyn Host, player: Uuid, _first: bool) {
    if !ess.state::<GeoState>().loaded {
        reload(ess, host);
    }
    if !ess.state::<GeoState>().enabled || ess.has(host, Sender::Player(player), "essentials.geoip.hide") {
        return;
    }
    let Some(address) = host.address(player).and_then(|a| a.parse::<IpAddr>().ok()) else { return };
    let name = host.name(player).unwrap_or_default();
    if ess.state::<GeoState>().reader.is_none() {
        let msg = literal(ess, "geoIpErrorOnJoin", &[Arg::Plain(name)]);
        log(ess, host, &msg);
        return;
    }
    let config = ess.state::<GeoState>().config.clone();
    let mut sb = String::new();
    let found = {
        let reader = ess.state::<GeoState>().reader.as_ref().expect("checked");
        reader.lookup(address).map(|record| {
            if config_bool(&config, "database.show-cities", false) {
                let city = reader.name(record.get("city"));
                let region = record.get("subdivisions").and_then(Value::as_array).and_then(|a| a.last()).and_then(|s| reader.name(Some(s)));
                let country = reader.name(record.get("country"));
                if let Some(c) = city {
                    sb.push_str(&c);
                    sb.push_str(", ");
                }
                if let Some(r) = region {
                    sb.push_str(&r);
                    sb.push_str(", ");
                }
                sb.push_str(&country.unwrap_or_else(|| "null".into()));
            } else {
                sb.push_str(&reader.name(record.get("country")).unwrap_or_else(|| "null".into()));
            }
        })
    };
    if found.is_none() {
        if is_local(address) {
            for p in host.online() {
                if ess.has(host, Sender::Player(p), "essentials.geoip.show") {
                    ess.send_tl(host, Sender::Player(p), "geoipCantFind", &[Arg::Plain(name.clone())]);
                }
            }
            return;
        }
        let msg = literal(ess, "cantReadGeoIpDB", &[]);
        log(ess, host, &format!("{msg} The address {address} is not in the database."));
    }
    if config_bool(&config, "show-on-whois", true) {
        ess.state::<GeoState>().locations.insert(player, sb.clone());
    }
    if config_bool(&config, "show-on-login", true) && !ess.is_vanished(player) {
        let args = [Arg::Plain(name), Arg::Plain(sb)];
        let msg = literal(ess, "geoipJoinFormat", &args);
        log(ess, host, &msg);
        for p in host.online() {
            if ess.has(host, Sender::Player(p), "essentials.geoip.show") {
                ess.send_tl(host, Sender::Player(p), "geoipJoinFormat", &args);
            }
        }
    }
}

/// checkIfLocal: the wildcard or loopback address (EssentialsX also asks the network interfaces, which we can't).
fn is_local(ip: IpAddr) -> bool {
    ip.is_unspecified() || ip.is_loopback()
}

/// User.getGeoLocation: where the player was found on their last join (None without a database or show-on-whois).
pub fn geo_location(ess: &mut Ess, player: Uuid) -> Option<String> {
    ess.state::<GeoState>().locations.get(&player).cloned()
}

/// The /whois line (`whoisGeoLocation`, MiniMessage) for the admin area. Commandwhois shows it only to the
/// console and to players with `essentials.geoip.show`: the caller checks the viewer.
pub fn whois_line(ess: &mut Ess, _host: &mut dyn Host, player: Uuid) -> Option<String> {
    let location = geo_location(ess, player)?;
    Some(ess.tl("whoisGeoLocation", &[Arg::Plain(location)]))
}

// ===== MaxMind DB reader ===========================================================================
// https://maxmind.github.io/MaxMind-DB/: a binary search tree over the address bits, then a data section of
// typed values, then metadata after the marker \xAB\xCD\xEFMaxMind.com.

const METADATA_MARKER: &[u8] = b"\xAB\xCD\xEFMaxMind.com";

pub struct Mmdb {
    bytes: Vec<u8>,
    pub node_count: u32,
    pub record_size: u32,
    pub ip_version: u32,
    pub database_type: String,
    data_start: usize,
    /// Name languages in order of preference (names.<locale>).
    pub locales: Vec<String>,
    /// Where IPv4 lookups start in an IPv6 tree (after 96 zero bits).
    ipv4_start: u32,
}

impl Mmdb {
    pub fn new(bytes: Vec<u8>) -> Option<Mmdb> {
        let at = bytes.windows(METADATA_MARKER.len()).rposition(|w| w == METADATA_MARKER)? + METADATA_MARKER.len();
        let (meta, _) = decode(&bytes[at..], 0, 0)?;
        let num = |k: &str| meta.get(k).and_then(Value::as_u64);
        let node_count = num("node_count")? as u32;
        let record_size = num("record_size")? as u32;
        if ![24, 28, 32].contains(&record_size) {
            return None;
        }
        let ip_version = num("ip_version").unwrap_or(6) as u32;
        let tree_size = (record_size as usize * 2 / 8) * node_count as usize;
        let data_start = tree_size + 16;
        if data_start > bytes.len() {
            return None;
        }
        let mut db = Mmdb {
            bytes,
            node_count,
            record_size,
            ip_version,
            database_type: meta.get("database_type").and_then(Value::as_str).unwrap_or("").to_string(),
            data_start,
            locales: vec!["en".into()],
            ipv4_start: 0,
        };
        if ip_version == 6 {
            let mut node = 0u32;
            for _ in 0..96 {
                if node >= node_count {
                    break;
                }
                node = db.record(node, 0)?;
            }
            db.ipv4_start = node;
        }
        Some(db)
    }

    fn record(&self, node: u32, side: u32) -> Option<u32> {
        let b = &self.bytes;
        let n = node as usize;
        match self.record_size {
            24 => {
                let o = n * 6 + side as usize * 3;
                let s = b.get(o..o + 3)?;
                Some(u32::from_be_bytes([0, s[0], s[1], s[2]]))
            }
            28 => {
                let o = n * 7;
                let s = b.get(o..o + 7)?;
                Some(if side == 0 {
                    (((s[3] & 0xF0) as u32) << 20) | u32::from_be_bytes([0, s[0], s[1], s[2]])
                } else {
                    (((s[3] & 0x0F) as u32) << 24) | u32::from_be_bytes([0, s[4], s[5], s[6]])
                })
            }
            _ => {
                let o = n * 8 + side as usize * 4;
                let s = b.get(o..o + 4)?;
                Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
            }
        }
    }

    /// The record for an address, or None when it isn't in the database.
    pub fn lookup(&self, ip: IpAddr) -> Option<Value> {
        let (bits, mut node): (Vec<u8>, u32) = match ip {
            IpAddr::V4(v4) => {
                if self.ip_version == 6 {
                    (v4.octets().to_vec(), self.ipv4_start)
                } else {
                    (v4.octets().to_vec(), 0)
                }
            }
            IpAddr::V6(v6) => {
                if self.ip_version == 4 {
                    (v6.to_ipv4_mapped().or_else(|| v6.to_ipv4())?.octets().to_vec(), 0)
                } else {
                    (v6.octets().to_vec(), 0)
                }
            }
        };
        let total = bits.len() * 8;
        let mut i = 0;
        while i < total && node < self.node_count {
            let bit = (bits[i / 8] >> (7 - (i % 8))) & 1;
            node = self.record(node, bit as u32)?;
            i += 1;
        }
        if node <= self.node_count {
            return None;
        }
        let offset = (node - self.node_count) as usize - 16;
        let data = self.bytes.get(self.data_start..)?;
        decode(data, offset, 0).map(|(v, _)| v)
    }

    /// `names` in the preferred locale (Country.getName()...).
    pub fn name(&self, place: Option<&Value>) -> Option<String> {
        let names = place?.get("names")?;
        self.locales.iter().find_map(|l| names.get(l).and_then(Value::as_str)).map(str::to_string)
    }
}

/// Decodes one value at `at` of a section (pointers are relative to the section start). Returns the value and
/// the offset after it.
fn decode(section: &[u8], at: usize, depth: u32) -> Option<(Value, usize)> {
    if depth > 64 {
        return None;
    }
    let ctrl = *section.get(at)?;
    let mut pos = at + 1;
    let mut kind = ctrl >> 5;
    if kind == 1 {
        // Pointer: follow it, the value after the pointer is what follows its own bytes.
        let ss = (ctrl >> 3) & 3;
        let v = (ctrl & 7) as usize;
        let take = |n: usize| -> Option<usize> { Some(section.get(pos..pos + n)?.iter().fold(0usize, |a, b| (a << 8) | *b as usize)) };
        let target = match ss {
            0 => (v << 8) | take(1)?,
            1 => ((v << 16) | take(2)?) + 2048,
            2 => ((v << 24) | take(3)?) + 526_336,
            _ => take(4)?,
        };
        pos += ss as usize + 1;
        let (value, _) = decode(section, target, depth + 1)?;
        return Some((value, pos));
    }
    if kind == 0 {
        kind = 7 + *section.get(pos)?;
        pos += 1;
    }
    let mut size = (ctrl & 0x1f) as usize;
    if size >= 29 {
        let n = size - 28;
        let extra = section.get(pos..pos + n)?.iter().fold(0usize, |a, b| (a << 8) | *b as usize);
        pos += n;
        size = match n {
            1 => 29 + extra,
            2 => 285 + extra,
            _ => 65_821 + extra,
        };
    }
    let uint = |bytes: &[u8]| bytes.iter().fold(0u128, |a, b| (a << 8) | *b as u128);
    match kind {
        2 => {
            let s = section.get(pos..pos + size)?;
            Some((Value::String(String::from_utf8_lossy(s).into_owned()), pos + size))
        }
        3 => {
            let s: [u8; 8] = section.get(pos..pos + 8)?.try_into().ok()?;
            Some((json!(f64::from_be_bytes(s)), pos + 8))
        }
        4 => {
            let s = section.get(pos..pos + size)?;
            Some((Value::Array(s.iter().map(|b| json!(b)).collect()), pos + size))
        }
        5 | 6 | 9 => {
            let s = section.get(pos..pos + size)?;
            Some((json!(uint(s) as u64), pos + size))
        }
        10 => {
            let s = section.get(pos..pos + size)?;
            let v = uint(s);
            let value = if v <= u64::MAX as u128 { json!(v as u64) } else { Value::String(v.to_string()) };
            Some((value, pos + size))
        }
        8 => {
            let s = section.get(pos..pos + size)?;
            let mut buf = [0u8; 4];
            buf[4 - size.min(4)..].copy_from_slice(&s[s.len() - size.min(4)..]);
            Some((json!(i32::from_be_bytes(buf)), pos + size))
        }
        7 => {
            let mut map = Map::new();
            for _ in 0..size {
                let (key, next) = decode(section, pos, depth + 1)?;
                let (value, next) = decode(section, next, depth + 1)?;
                pos = next;
                map.insert(key.as_str()?.to_string(), value);
            }
            Some((Value::Object(map), pos))
        }
        11 => {
            let mut items = Vec::with_capacity(size.min(1024));
            for _ in 0..size {
                let (value, next) = decode(section, pos, depth + 1)?;
                pos = next;
                items.push(value);
            }
            Some((Value::Array(items), pos))
        }
        14 => Some((Value::Bool(size != 0), pos)),
        15 => {
            let s: [u8; 4] = section.get(pos..pos + 4)?.try_into().ok()?;
            Some((json!(f32::from_be_bytes(s) as f64), pos + 4))
        }
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::game::ess_testkit::*;

    // ----- A tiny .mmdb writer (just enough of the format for the reader's tests) -----
    pub fn ctrl(kind: u8, size: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let (sz, extra): (u8, Vec<u8>) = if size < 29 {
            (size as u8, vec![])
        } else if size < 285 {
            (29, vec![(size - 29) as u8])
        } else {
            let n = size - 285;
            (30, vec![(n >> 8) as u8, n as u8])
        };
        if kind <= 7 {
            out.push((kind << 5) | sz);
        } else {
            out.push(sz);
            out.push(kind - 7);
        }
        out.extend(extra);
        out
    }
    pub fn s(text: &str) -> Vec<u8> {
        let mut out = ctrl(2, text.len());
        out.extend(text.as_bytes());
        out
    }
    pub fn u32v(v: u32) -> Vec<u8> {
        let bytes: Vec<u8> = v.to_be_bytes().iter().copied().skip_while(|b| *b == 0).collect();
        let mut out = ctrl(6, bytes.len());
        out.extend(bytes);
        out
    }
    pub fn u16v(v: u16) -> Vec<u8> {
        let mut out = ctrl(5, 2);
        out.extend(v.to_be_bytes());
        out
    }
    pub fn dbl(v: f64) -> Vec<u8> {
        let mut out = ctrl(3, 8);
        out.extend(v.to_be_bytes());
        out
    }
    pub fn boolean(v: bool) -> Vec<u8> {
        ctrl(14, v as usize)
    }
    pub fn map(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut out = ctrl(7, entries.len());
        for (k, v) in entries {
            out.extend(s(k));
            out.extend(v);
        }
        out
    }
    pub fn array(items: &[Vec<u8>]) -> Vec<u8> {
        let mut out = ctrl(11, items.len());
        for i in items {
            out.extend(i);
        }
        out
    }
    pub fn pointer(offset: usize) -> Vec<u8> {
        assert!(offset < 2048);
        vec![(1 << 5) | ((offset >> 8) as u8 & 7), offset as u8]
    }

    enum Child {
        Empty,
        Node(usize),
        Data(usize),
    }

    /// Builds a database: (network bits, prefix length, data offset) entries over `data`.
    pub fn build(ip_version: u32, record_size: u32, networks: &[(Vec<u8>, usize, usize)], data: &[u8]) -> Vec<u8> {
        let mut nodes: Vec<[Child; 2]> = vec![[Child::Empty, Child::Empty]];
        for (bits, len, offset) in networks {
            let mut node = 0;
            for i in 0..*len {
                let bit = ((bits[i / 8] >> (7 - (i % 8))) & 1) as usize;
                if i + 1 == *len {
                    nodes[node][bit] = Child::Data(*offset);
                } else {
                    node = match nodes[node][bit] {
                        Child::Node(n) => n,
                        _ => {
                            nodes.push([Child::Empty, Child::Empty]);
                            let n = nodes.len() - 1;
                            nodes[node][bit] = Child::Node(n);
                            n
                        }
                    };
                }
            }
        }
        let count = nodes.len() as u32;
        let value = |c: &Child| match c {
            Child::Empty => count,
            Child::Node(n) => *n as u32,
            Child::Data(o) => count + 16 + *o as u32,
        };
        let mut out = Vec::new();
        for [l, r] in &nodes {
            let (l, r) = (value(l), value(r));
            match record_size {
                24 => {
                    out.extend(&l.to_be_bytes()[1..]);
                    out.extend(&r.to_be_bytes()[1..]);
                }
                28 => {
                    out.extend(&l.to_be_bytes()[1..]);
                    out.push((((l >> 24) & 0xF) << 4) as u8 | ((r >> 24) & 0xF) as u8);
                    out.extend(&r.to_be_bytes()[1..]);
                }
                _ => {
                    out.extend(l.to_be_bytes());
                    out.extend(r.to_be_bytes());
                }
            }
        }
        out.extend([0u8; 16]);
        out.extend(data);
        out.extend(METADATA_MARKER);
        out.extend(map(&[
            ("node_count", u32v(count)),
            ("record_size", u16v(record_size as u16)),
            ("ip_version", u16v(ip_version as u16)),
            ("database_type", s("GeoIP2-Test")),
            ("binary_format_major_version", u16v(2)),
        ]));
        out
    }

    /// A v4 network inside an IPv6 tree (::a.b.c.d/96+len).
    pub fn v4_in_v6(octets: [u8; 4]) -> Vec<u8> {
        let mut bits = vec![0u8; 12];
        bits.extend(octets);
        bits
    }

    /// A country database (IPv6 tree, 28-bit records): 81.2.69.0/24 = United Kingdom (names via a pointer),
    /// 2.125.0.0/16 = Germany.
    pub fn country_db() -> Vec<u8> {
        let mut data = Vec::new();
        let names_gb = data.len();
        data.extend(map(&[("en", s("United Kingdom")), ("de", s("Vereinigtes Königreich"))]));
        let gb = data.len();
        data.extend(map(&[("country", map(&[("iso_code", s("GB")), ("names", pointer(names_gb))])), ("is_eu", boolean(false))]));
        let de = data.len();
        data.extend(map(&[("country", map(&[("iso_code", s("DE")), ("names", map(&[("en", s("Germany"))]))])), ("lat", dbl(51.5))]));
        build(6, 28, &[(v4_in_v6([81, 2, 69, 0]), 96 + 24, gb), (v4_in_v6([2, 125, 0, 0]), 96 + 16, de)], &data)
    }

    /// A city database (IPv4 tree, 24-bit records): 81.2.69.0/24 = London, England, United Kingdom; 89.160.20.0/24
    /// has no city.
    pub fn city_db() -> Vec<u8> {
        let mut data = Vec::new();
        let london = data.len();
        data.extend(map(&[
            ("city", map(&[("names", map(&[("en", s("London"))]))])),
            ("subdivisions", array(&[map(&[("names", map(&[("en", s("England"))]))])])),
            ("country", map(&[("names", map(&[("en", s("United Kingdom"))]))])),
        ]));
        let sweden = data.len();
        data.extend(map(&[("country", map(&[("names", map(&[("en", s("Sweden"))]))]))]));
        build(4, 24, &[(vec![81, 2, 69, 0], 24, london), (vec![89, 160, 20, 0], 24, sweden)], &data)
    }

    #[test]
    fn reader_decodes_country_and_city_databases() {
        let db = Mmdb::new(country_db()).expect("country db");
        assert_eq!((db.record_size, db.ip_version), (28, 6));
        assert_eq!(db.database_type, "GeoIP2-Test");
        let gb = db.lookup("81.2.69.160".parse().unwrap()).expect("found");
        assert_eq!(db.name(gb.get("country")).as_deref(), Some("United Kingdom"));
        assert_eq!(gb["country"]["iso_code"], "GB");
        assert_eq!(gb["is_eu"], false);
        let de = db.lookup("2.125.160.216".parse().unwrap()).expect("found");
        assert_eq!(db.name(de.get("country")).as_deref(), Some("Germany"));
        assert_eq!(de["lat"], 51.5);
        assert!(db.lookup("8.8.8.8".parse().unwrap()).is_none());
        // An IPv4-mapped IPv6 address walks the same ::/96 subtree in a v6 tree only when written as ::a.b.c.d.
        assert!(db.lookup("::5102:45a0".parse().unwrap()).is_some());

        let city = Mmdb::new(city_db()).expect("city db");
        assert_eq!((city.record_size, city.ip_version), (24, 4));
        let london = city.lookup("81.2.69.1".parse().unwrap()).unwrap();
        assert_eq!(city.name(london.get("city")).as_deref(), Some("London"));
        assert!(city.lookup("1.1.1.1".parse().unwrap()).is_none());
        assert!(city.lookup("::ffff:81.2.69.1".parse().unwrap()).is_some());
    }

    #[test]
    fn reader_handles_32_bit_records_and_locales() {
        let mut data = Vec::new();
        data.extend(map(&[("country", map(&[("names", map(&[("en", s("France")), ("fr", s("France")), ("de", s("Frankreich"))]))]))]));
        let mut db = Mmdb::new(build(4, 32, &[(vec![10, 0, 0, 0], 8, 0)], &data)).unwrap();
        assert_eq!(db.record_size, 32);
        let r = db.lookup("10.1.2.3".parse().unwrap()).unwrap();
        db.locales = vec!["de".into(), "en".into()];
        assert_eq!(db.name(r.get("country")).as_deref(), Some("Frankreich"));
        db.locales = vec!["pt-BR".into(), "en".into()];
        assert_eq!(db.name(r.get("country")).as_deref(), Some("France"));
        assert!(Mmdb::new(b"not a database".to_vec()).is_none());
    }

    #[test]
    fn decoder_reads_every_type() {
        let mut v = Vec::new();
        v.extend(map(&[
            ("u16", u16v(513)),
            ("u32", u32v(70000)),
            ("long", {
                let mut s = s(&"x".repeat(40));
                s.truncate(s.len());
                s
            }),
        ]));
        let (value, end) = decode(&v, 0, 0).unwrap();
        assert_eq!(end, v.len());
        assert_eq!(value["u16"], 513);
        assert_eq!(value["u32"], 70000);
        assert_eq!(value["long"].as_str().unwrap().len(), 40);
        // int32, uint64, uint128 (extended types), float, bytes.
        let mut x = ctrl(8, 4);
        x.extend((-5i32).to_be_bytes());
        assert_eq!(decode(&x, 0, 0).unwrap().0, -5);
        let mut x = ctrl(9, 8);
        x.extend(u64::MAX.to_be_bytes());
        assert_eq!(decode(&x, 0, 0).unwrap().0, json!(u64::MAX));
        let mut x = ctrl(10, 16);
        x.extend(u128::MAX.to_be_bytes());
        assert_eq!(decode(&x, 0, 0).unwrap().0, json!(u128::MAX.to_string()));
        let mut x = ctrl(15, 4);
        x.extend(1.5f32.to_be_bytes());
        assert_eq!(decode(&x, 0, 0).unwrap().0, 1.5);
        let mut x = ctrl(4, 2);
        x.extend([7, 9]);
        assert_eq!(decode(&x, 0, 0).unwrap().0, json!([7, 9]));
    }

    fn server(config: &str, db: Option<(&str, Vec<u8>)>) -> TestServer {
        let t = TestServer::new();
        let dir = t.ess.dir.join("geoip");
        std::fs::create_dir_all(&dir).unwrap();
        if !config.is_empty() {
            std::fs::write(dir.join("config.yml"), config).unwrap();
        }
        if let Some((name, bytes)) = db {
            std::fs::write(dir.join(name), bytes).unwrap();
        }
        t
    }

    /// A player joins from an address (the address is set before Essentials sees the join).
    fn join_from(t: &mut TestServer, name: &str, ip: &str) -> Uuid {
        let uuid = t.host.add_player(name);
        t.host.p(uuid).address = ip.to_string();
        t.ess.on_join(&mut t.host, uuid);
        uuid
    }

    /// An operator (essentials.geoip.show, and .hide so their own join isn't announced) joins.
    fn join_admin(t: &mut TestServer) -> Uuid {
        let uuid = t.host.add_player("Admin");
        t.host.p(uuid).op = 4;
        t.ess.on_join(&mut t.host, uuid);
        t.said(uuid);
        uuid
    }

    const BASE: &str = "database:\n  show-cities: false\n  download-if-missing: false\nshow-on-login: true\nshow-on-whois: true\nenable-locale: true\n";

    #[test]
    fn join_message_goes_to_players_with_geoip_show() {
        let mut t = server(BASE, Some(("GeoIP2-Country.mmdb", country_db())));
        let admin = join_admin(&mut t);
        let bob = t.join("Bob");
        t.said(admin);
        t.said(bob);
        t.host.said(Sender::Console);
        let carol = join_from(&mut t, "Carol", "81.2.69.160");
        assert_eq!(t.said(admin), ["Player Carol comes from United Kingdom."]);
        // The default group has essentials.geoip.show (DEFAULT_PERMISSIONS), the joiner included.
        assert_eq!(t.said(bob), ["Player Carol comes from United Kingdom."]);
        assert_eq!(t.said(carol), ["Player Carol comes from United Kingdom."]);
        t.permissions("users:\n  bob:\n    permissions: [-essentials.geoip.show]\n");
        join_from(&mut t, "Dave", "2.125.160.216");
        assert_eq!(t.said(admin), ["Player Dave comes from Germany."]);
        assert!(t.said(bob).is_empty());
        assert_eq!(t.host.said(Sender::Console), ["Player Carol comes from United Kingdom.", "Player Dave comes from Germany."]);
        assert_eq!(geo_location(&mut t.ess, carol).as_deref(), Some("United Kingdom"));
        assert_eq!(
            whois_line(&mut t.ess, &mut t.host, carol).map(|l| crate::game::ess_text::plain(&t.ess.render(&l))).as_deref(),
            Some(" - Location: United Kingdom")
        );
    }

    #[test]
    fn hidden_players_and_geoip_hide_are_not_announced() {
        let mut t = server(BASE, Some(("GeoIP2-Country.mmdb", country_db())));
        let admin = join_admin(&mut t);
        // Ops have essentials.geoip.hide by default.
        let op = t.host.add_player("Oscar");
        t.host.p(op).op = 4;
        t.host.p(op).address = "81.2.69.1".into();
        t.ess.on_join(&mut t.host, op);
        assert!(t.said(admin).is_empty());
        assert_eq!(geo_location(&mut t.ess, op), None);
        // Vanished players: located (whois) but not announced.
        let dave = t.host.add_player("Dave");
        t.host.p(dave).address = "2.125.160.216".into();
        t.ess.users.user(dave).set("vanished", json!(true));
        t.ess.on_join(&mut t.host, dave);
        assert!(t.said(admin).is_empty());
        assert_eq!(geo_location(&mut t.ess, dave).as_deref(), Some("Germany"));
    }

    #[test]
    fn show_on_login_and_show_on_whois_switches() {
        let cfg = BASE.replace("show-on-login: true", "show-on-login: false").replace("show-on-whois: true", "show-on-whois: false");
        let mut t = server(&cfg, Some(("GeoIP2-Country.mmdb", country_db())));
        let admin = join_admin(&mut t);
        let carol = join_from(&mut t, "Carol", "81.2.69.160");
        assert!(t.said(admin).is_empty());
        assert_eq!(whois_line(&mut t.ess, &mut t.host, carol), None);
    }

    #[test]
    fn cities_show_city_region_and_country() {
        let cfg = BASE.replace("show-cities: false", "show-cities: true");
        let mut t = server(&cfg, Some(("GeoIP2-City.mmdb", city_db())));
        let admin = join_admin(&mut t);
        join_from(&mut t, "Carol", "81.2.69.160");
        assert_eq!(t.said(admin), ["Player Carol comes from London, England, United Kingdom."]);
        join_from(&mut t, "Sven", "89.160.20.112");
        assert_eq!(t.said(admin), ["Player Sven comes from Sweden."]);
    }

    #[test]
    fn local_and_unknown_addresses() {
        let mut t = server(BASE, Some(("GeoIP2-Country.mmdb", country_db())));
        let admin = join_admin(&mut t);
        // FakeHost addresses are loopback: "an unknown country".
        t.join("Bob");
        assert_eq!(t.said(admin), ["Player Bob comes from an unknown country."]);
        // A public address missing from the database: logged, then announced with an empty place (as EssentialsX).
        t.host.said(Sender::Console);
        join_from(&mut t, "Eve", "8.8.8.8");
        assert_eq!(t.said(admin), ["Player Eve comes from ."]);
        let console = t.host.said(Sender::Console);
        assert!(console[0].starts_with("Failed to read GeoIP database!"), "{console:?}");
    }

    #[test]
    fn missing_database_logs_and_downloads_only_with_a_license_key() {
        // No database and no license key: GeoIP is off and says nothing (the console stays quiet for everyone).
        let mut t = server(BASE, None);
        let admin = join_admin(&mut t);
        assert!(t.host.said(Sender::Console).is_empty());
        join_from(&mut t, "Carol", "81.2.69.160");
        assert!(t.said(admin).is_empty());
        assert!(t.host.said(Sender::Console).is_empty());
        assert!(t.host.http_requests.is_empty());

        // A key but download-if-missing off: EssentialsX's "can't find" and the join error.
        let mut t = server("database:\n  download-if-missing: false\n  license-key: 'ABC'\nenable-locale: true\n", None);
        let admin = t.host.add_player("Admin");
        t.host.p(admin).op = 4;
        t.ess.on_join(&mut t.host, admin);
        assert_eq!(t.host.said(Sender::Console), ["Can't find GeoIP database!"]);
        join_from(&mut t, "Carol", "81.2.69.160");
        assert!(t.said(admin).is_empty());
        assert_eq!(
            t.host.said(Sender::Console),
            ["Unable to fetch GeoIP data for Carol. Please ensure that your license key and configuration are correct."]
        );
        assert!(t.host.http_requests.is_empty());

        // With a key: the request goes out; the answer tells the admin where to put the file.
        let mut t = server("database:\n  download-if-missing: true\n  license-key: 'ABC'\nenable-locale: true\n", None);
        t.join("Admin");
        let console = t.host.said(Sender::Console);
        assert!(console.contains(&"Downloading GeoIP database... this might take a while (country: 1.7 MB, city: 30MB)".to_string()), "{console:?}");
        let (id, method, url, _, _) = t.host.http_requests[0].clone();
        assert_eq!(method, "GET");
        assert_eq!(url, "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key=ABC&suffix=tar.gz");
        assert!(on_http(&mut t.ess, &mut t.host, id, 200, "binary"));
        assert!(!on_http(&mut t.ess, &mut t.host, 42, 200, ""));
        let console = t.host.said(Sender::Console);
        assert!(console[0].starts_with("Can't find GeoIP database! Download it from MaxMind and place it at "), "{console:?}");
        assert!(console[0].ends_with("GeoIP2-Country.mmdb"));
        on_http(&mut t.ess, &mut t.host, id, 401, "");
        assert_eq!(t.host.said(Sender::Console)[0], "Failed to open connection.");

        // The city database's URL with show-cities.
        let mut t = server("database:\n  show-cities: true\n  download-if-missing: true\n  license-key: 'K'\nenable-locale: true\n", None);
        t.join("Admin");
        assert!(t.host.http_requests[0].2.contains("edition_id=GeoLite2-City&license_key=K"));

        // An empty or invalid URL.
        let mut t = server("database:\n  download-if-missing: true\n  download-url: ''\n  license-key: 'ABC'\nenable-locale: true\n", None);
        t.join("Admin");
        assert!(t.host.said(Sender::Console).contains(&"GeoIP download url is empty.".to_string()));
        let mut t = server("database:\n  download-if-missing: true\n  download-url: 'ftp:/x'\n  license-key: 'ABC'\nenable-locale: true\n", None);
        t.join("Admin");
        assert!(t.host.said(Sender::Console).contains(&"GeoIP download url is invalid.".to_string()));
    }

    #[test]
    fn old_configs_are_migrated() {
        let mut t = server("database:\n  show-cities: false\n  download-if-missing: false\n", Some(("GeoIP2-Country.mmdb", country_db())));
        t.join("Admin");
        let cfg = t.ess.read_yaml("geoip/config.yml");
        assert_eq!(cfg["enable-locale"], true);
        assert_eq!(cfg["database"]["update"]["by-every-x-days"], 30);
        assert_eq!(cfg["database"]["license-key"], "");
    }

    #[test]
    fn default_config_without_database_is_off() {
        // The data folder's default geoip/config.yml (download-if-missing true, no key): nothing announced.
        let mut t = TestServer::new();
        let admin = join_admin(&mut t);
        join_from(&mut t, "Carol", "81.2.69.160");
        assert!(t.said(admin).is_empty());
        assert!(t.host.http_requests.is_empty());
    }
}
