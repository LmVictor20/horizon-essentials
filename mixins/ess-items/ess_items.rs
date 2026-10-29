// horizon-essentials area `items` (pure logic, standalone-tested): items, kits and inventories. A port of
// EssentialsX's item commands (Command*.java), Kit/Kits, MetaItemStack (the item meta syntax of kits and /give),
// FlatItemDb (items.json aliases, generated into ess_items_db) and craftbukkit/Inventories.
//
// Items are `Item { id, count, components }` with vanilla data components as JSON, keys without the
// `minecraft:` namespace (`custom_name`, `lore`, `enchantments`, `damage`, `potion_contents`, `fireworks`...).
// Inventories use Bukkit's slot numbers (0-8 hotbar, 9-35 main, 36-39 boots..helmet, 40 offhand).
//
// For other areas: `item_stack` (an item by name), `parse_item_line` (a kits.yml/`/give`-style line),
// `give_kit` (a kit without permission or delay checks), `add_items` (give, dropping or not what doesn't fit),
// `item_display_name`. Hooks call `on_left_click`, `on_attack_player` (powertools) and `on_use_item` (unlimited).

use crate::game::ess_core::*;
use crate::game::{ess_items_db as db, ess_text};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::OnceLock;

pub fn register(ess: &mut Ess) {
    let cmds: &[(&'static str, CmdFn, &'static [(&'static str, &'static str)])] = &[
        ("anvil", cmd_anvil, &[]),
        ("book", cmd_book, U_BOOK),
        ("cartographytable", cmd_cartographytable, &[]),
        ("clearinventory", cmd_clearinventory, U_CLEARINVENTORY),
        ("clearinventoryconfirmtoggle", cmd_clearinventoryconfirmtoggle, &[]),
        ("condense", cmd_condense, U_CONDENSE),
        ("createkit", cmd_createkit, U_CREATEKIT),
        ("delkit", cmd_delkit, U_DELKIT),
        ("disposal", cmd_disposal, &[]),
        ("enchant", cmd_enchant, U_ENCHANT),
        ("enderchest", cmd_enderchest, U_ENDERCHEST),
        ("exp", cmd_exp, U_EXP),
        ("firework", cmd_firework, U_FIREWORK),
        ("give", cmd_give, U_GIVE),
        ("grindstone", cmd_grindstone, &[]),
        ("hat", cmd_hat, U_HAT),
        ("item", cmd_item, U_ITEM),
        ("itemdb", cmd_itemdb, U_ITEMDB),
        ("itemlore", cmd_itemlore, U_ITEMLORE),
        ("itemname", cmd_itemname, U_ITEMNAME),
        ("kit", cmd_kit, U_KIT),
        ("kitreset", cmd_kitreset, U_KITRESET),
        ("loom", cmd_loom, &[]),
        ("more", cmd_more, U_MORE),
        ("potion", cmd_potion, U_POTION),
        ("powertool", cmd_powertool, U_POWERTOOL),
        ("powertoollist", cmd_powertoollist, &[]),
        ("powertooltoggle", cmd_powertooltoggle, &[]),
        ("recipe", cmd_recipe, U_RECIPE),
        ("repair", cmd_repair, U_REPAIR),
        ("showkit", cmd_showkit, U_SHOWKIT),
        ("skull", cmd_skull, U_SKULL),
        ("smithingtable", cmd_smithingtable, &[]),
        ("stonecutter", cmd_stonecutter, &[]),
        ("unlimited", cmd_unlimited, U_UNLIMITED),
        ("workbench", cmd_workbench, &[]),
    ];
    for (name, run, usages) in cmds {
        ess.register(Command { name, run: *run, tab: tab_for(name), usages });
    }
    ess.listeners.container_click.push(on_container_click);
    ess.listeners.container_close.push(on_container_close);
    ess.listeners.tick.push(poll_closed_containers);
    ess.listeners.quit.push(|ess, _, p| {
        ess.state::<ItemsState>().confirming_clear.remove(&p);
    });
}

const U_BOOK: &[(&str, &str)] = &[("/<command>", "bookCommandUsage1Description"), ("/<command> author <author>", "bookCommandUsage2Description"), ("/<command> title <title>", "bookCommandUsage3Description")];
const U_CLEARINVENTORY: &[(&str, &str)] = &[("/<command>", "clearinventoryCommandUsage1Description"), ("/<command> <player>", "clearinventoryCommandUsage2Description"), ("/<command> <player> <item> [amount]", "clearinventoryCommandUsage3Description")];
const U_CONDENSE: &[(&str, &str)] = &[("/<command>", "condenseCommandUsage1Description"), ("/<command> <item>", "condenseCommandUsage2Description")];
const U_CREATEKIT: &[(&str, &str)] = &[("/<command> <kitname> <delay>", "createkitCommandUsage1Description")];
const U_DELKIT: &[(&str, &str)] = &[("/<command> <kit>", "delkitCommandUsage1Description")];
const U_ENCHANT: &[(&str, &str)] = &[("/<command> <enchantment name> [level]", "enchantCommandUsage1Description")];
const U_ENDERCHEST: &[(&str, &str)] = &[("/<command>", "enderchestCommandUsage1Description"), ("/<command> <player>", "enderchestCommandUsage2Description")];
const U_EXP: &[(&str, &str)] = &[("/<command> give <player> <amount>", "expCommandUsage1Description"), ("/<command> set <playername> <amount>", "expCommandUsage2Description"), ("/<command> show <playername>", "expCommandUsage3Description"), ("/<command> reset <playername>", "expCommandUsage4Description")];
const U_FIREWORK: &[(&str, &str)] = &[("/<command> clear", "fireworkCommandUsage1Description"), ("/<command> power <amount>", "fireworkCommandUsage2Description"), ("/<command> fire [amount]", "fireworkCommandUsage3Description"), ("/<command> <meta>", "fireworkCommandUsage4Description")];
const U_GIVE: &[(&str, &str)] = &[("/<command> <player> <item> [amount]", "giveCommandUsage1Description"), ("/<command> <player> <item> <amount> <meta>", "giveCommandUsage2Description")];
const U_HAT: &[(&str, &str)] = &[("/<command>", "hatCommandUsage1Description"), ("/<command> remove", "hatCommandUsage2Description")];
const U_ITEM: &[(&str, &str)] = &[("/<command> <item> [amount]", "itemCommandUsage1Description"), ("/<command> <item> <amount> <meta>", "itemCommandUsage2Description")];
const U_ITEMDB: &[(&str, &str)] = &[("/<command> <item>", "itemdbCommandUsage1Description")];
const U_ITEMLORE: &[(&str, &str)] = &[("/<command> add [text]", "itemloreCommandUsage1Description"), ("/<command> set <line number> <text>", "itemloreCommandUsage2Description"), ("/<command> clear", "itemloreCommandUsage3Description")];
const U_ITEMNAME: &[(&str, &str)] = &[("/<command>", "itemnameCommandUsage1Description"), ("/<command> <name>", "itemnameCommandUsage2Description")];
const U_KIT: &[(&str, &str)] = &[("/<command>", "kitCommandUsage1Description"), ("/<command> <kit> [player]", "kitCommandUsage2Description")];
const U_KITRESET: &[(&str, &str)] = &[("/<command> <kit> [player]", "kitresetCommandUsage1Description")];
const U_MORE: &[(&str, &str)] = &[("/<command> [amount]", "moreCommandUsage1Description")];
const U_POTION: &[(&str, &str)] = &[("/<command> clear", "potionCommandUsage1Description"), ("/<command> apply", "potionCommandUsage2Description"), ("/<command> effect:<effect> power:<power> duration:<duration>", "potionCommandUsage3Description")];
const U_POWERTOOL: &[(&str, &str)] = &[("/<command> l:", "powertoolCommandUsage1Description"), ("/<command> d:", "powertoolCommandUsage2Description"), ("/<command> r:<cmd>", "powertoolCommandUsage3Description"), ("/<command> <cmd>", "powertoolCommandUsage4Description"), ("/<command> a:<cmd>", "powertoolCommandUsage5Description")];
const U_RECIPE: &[(&str, &str)] = &[("/<command> <<item>|hand> [page]", "recipeCommandUsage1Description")];
const U_REPAIR: &[(&str, &str)] = &[("/<command>", "repairCommandUsage1Description"), ("/<command> all", "repairCommandUsage2Description")];
const U_SHOWKIT: &[(&str, &str)] = &[("/<command> <kitname>", "showkitCommandUsage1Description")];
const U_SKULL: &[(&str, &str)] = &[("/<command>", "skullCommandUsage1Description"), ("/<command> <player>", "skullCommandUsage2Description"), ("/<command> <texture>", "skullCommandUsage3Description"), ("/<command> <owner> <player>", "skullCommandUsage4Description"), ("/<command> <texture> <player>", "skullCommandUsage5Description")];
const U_UNLIMITED: &[(&str, &str)] = &[("/<command> list [player]", "unlimitedCommandUsage1Description"), ("/<command> <item> [player]", "unlimitedCommandUsage2Description"), ("/<command> clear [player]", "unlimitedCommandUsage3Description")];

/// In-memory state (User's transient fields in EssentialsX).
#[derive(Default)]
struct ItemsState {
    /// The /clearinventory line waiting for its confirmation (User.confirmingClearCommand).
    confirming_clear: HashMap<Uuid, String>,
}

// ===== Small helpers ==============================================================================

fn tl(key: &'static str, args: Vec<Arg>) -> CmdError {
    CmdError::Tl(key, args)
}
fn plain(s: impl Into<String>) -> Arg {
    Arg::Plain(s.into())
}
fn ns(id: &str) -> String {
    if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") }
}
fn bare(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}
/// Material.toString(): `DIAMOND_SWORD`.
fn mat_upper(item: &Item) -> String {
    bare(&item.id).to_ascii_uppercase()
}
/// `diamondsword` (item permission and blacklist names).
fn mat_compact(item: &Item) -> String {
    bare(&item.id).replace('_', "")
}
/// `diamond sword` (messages).
fn mat_spaced(item: &Item) -> String {
    bare(&item.id).replace('_', " ")
}
fn is_int(s: &str) -> bool {
    s.parse::<i32>().is_ok()
}
fn comps(item: &mut Item) -> &mut Map<String, Value> {
    if !item.components.is_object() {
        item.components = json!({});
    }
    item.components.as_object_mut().expect("an object")
}
fn comp<'a>(item: &'a Item, key: &str) -> Option<&'a Value> {
    item.components.get(key).filter(|v| !v.is_null())
}
fn set_comp(item: &mut Item, key: &str, value: Option<Value>) {
    match value {
        Some(v) => {
            comps(item).insert(key.to_string(), v);
        }
        None => {
            comps(item).remove(key);
        }
    }
}
fn same_components(a: &Value, b: &Value) -> bool {
    let empty = |v: &Value| v.is_null() || v.as_object().is_some_and(Map::is_empty);
    (empty(a) && empty(b)) || a == b
}
/// ItemStack.isSimilar: the same item and components, any count.
pub fn similar(a: &Item, b: &Item) -> bool {
    a.id == b.id && same_components(&a.components, &b.components)
}
fn damage_of(item: &Item) -> i64 {
    comp(item, "damage").and_then(Value::as_i64).unwrap_or(0)
}
fn enchants_of(item: &Item) -> Map<String, Value> {
    comp(item, "enchantments").and_then(Value::as_object).cloned().unwrap_or_default()
}
fn display_name(ctx: &mut Ctx, player: Uuid) -> String {
    ctx.name_of(player)
}
fn held(host: &mut dyn Host, player: Uuid) -> (usize, Option<Item>) {
    let slot = host.held_slot(player).min(8);
    let item = host.inventory(player).get(slot).cloned().flatten().filter(|i| i.count > 0 && bare(&i.id) != "air");
    (slot, item)
}
/// Bukkit's matchPlayer: an exact name, else every online player whose name contains the text.
fn match_players(ctx: &mut Ctx, search: &str) -> Vec<Uuid> {
    let s = search.to_lowercase();
    let online = ctx.host.online();
    let mut contains = Vec::new();
    for p in online {
        let name = ctx.host.name(p).unwrap_or_default().to_lowercase();
        if name == s {
            return vec![p];
        }
        if name.contains(&s) && ctx.ess.can_see(ctx.host, ctx.sender, p) {
            contains.push(p);
        }
    }
    contains
}
/// EssentialsLoopCommand's player matching: `*`/`**` is everyone online, else one online player.
fn loop_players(ctx: &mut Ctx, search: &str) -> Result<Vec<Uuid>, CmdError> {
    if search == "*" || search == "**" {
        let online = ctx.host.online();
        let mut out = Vec::new();
        for p in online {
            if ctx.ess.can_see(ctx.host, ctx.sender, p) {
                out.push(p);
            }
        }
        return if out.is_empty() { Err(CmdError::PlayerNotFound) } else { Ok(out) };
    }
    Ok(vec![ctx.online_player(search)?])
}

/// User.isAuthorizedExact: a node set for the player itself, not granted through a broad wildcard
/// (`essentials.*`, `*`): Some(true) set, Some(false) negated, None unset.
fn exact_perm(ctx: &mut Ctx, node: &str, probe: &str) -> Option<bool> {
    let Sender::Player(p) = ctx.sender else { return None };
    let name = ctx.host.name(p).unwrap_or_default();
    let got = ctx.ess.perms.configured(p, &name, node)?;
    if got && ctx.ess.perms.configured(p, &name, probe) == Some(true) {
        return None;
    }
    Some(got)
}

// ===== Text ==========================================================================================

fn style_json(span: &Span) -> Value {
    let mut o = Map::new();
    o.insert("text".into(), json!(span.text));
    let st = &span.style;
    if let Some(c) = &st.color {
        o.insert("color".into(), json!(c));
    }
    for (k, on) in [("bold", st.bold), ("italic", st.italic), ("underlined", st.underlined), ("strikethrough", st.strikethrough), ("obfuscated", st.obfuscated)] {
        if on {
            o.insert(k.into(), json!(true));
        }
    }
    Value::Object(o)
}

/// A text component from a string with `§` codes (Bukkit's legacy display names and lore).
pub fn legacy_component(s: &str) -> Value {
    let mini = ess_text::legacy_to_mini(&ess_text::escape_tags(s), false);
    let spans = ess_text::parse_mini(&mini, &ess_text::Palette::default());
    match spans.as_slice() {
        [] => json!(""),
        [one] if one.style == ess_text::Style::default() => json!(one.text),
        _ => json!({"text": "", "extra": spans.iter().map(style_json).collect::<Vec<_>>()}),
    }
}

/// The plain text of a text component (a string, `{text, extra}`, a list, or `{raw: ...}`).
pub fn component_plain(c: &Value) -> String {
    match c {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(component_plain).collect(),
        Value::Object(o) => {
            if let Some(raw) = o.get("raw") {
                return component_plain(raw);
            }
            let mut s = o.get("text").and_then(Value::as_str).unwrap_or("").to_string();
            if let Some(Value::Array(extra)) = o.get("extra") {
                for e in extra {
                    s.push_str(&component_plain(e));
                }
            }
            s
        }
        _ => String::new(),
    }
}

/// A text component back to `&` codes (FormatUtil.unformatString of a display name, for /createkit).
fn component_legacy(c: &Value) -> String {
    component_codes(c, '&')
}

/// A text component as a string with `§` codes: Horizon keeps custom names as plain strings, and vanilla
/// clients still draw `§` codes inside literal text (how Bukkit's legacy display names look).
pub fn component_section(c: &Value) -> String {
    component_codes(c, '§')
}

fn component_codes(c: &Value, mark: char) -> String {
    fn walk(c: &Value, out: &mut String, mark: char) {
        match c {
            Value::String(s) => out.push_str(s),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out, mark)),
            Value::Object(o) => {
                if let Some(raw) = o.get("raw") {
                    return walk(raw, out, mark);
                }
                let text = o.get("text").and_then(Value::as_str).unwrap_or("");
                if let Some(color) = o.get("color").and_then(Value::as_str) {
                    if let Some(i) = ess_text::COLOR_NAMES.iter().position(|n| *n == color) {
                        out.push(mark);
                        out.push("0123456789abcdef".as_bytes()[i] as char);
                    } else if let Some(hex) = color.strip_prefix('#').filter(|h| h.len() == 6) {
                        out.push(mark);
                        out.push('x');
                        for ch in hex.chars() {
                            out.push(mark);
                            out.push(ch);
                        }
                    }
                }
                for (k, code) in [("obfuscated", 'k'), ("bold", 'l'), ("strikethrough", 'm'), ("underlined", 'n'), ("italic", 'o')] {
                    if o.get(k).and_then(Value::as_bool) == Some(true) {
                        out.push(mark);
                        out.push(code);
                    }
                }
                out.push_str(text);
                if let Some(Value::Array(extra)) = o.get("extra") {
                    extra.iter().for_each(|x| walk(x, out, mark));
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    walk(c, &mut out, mark);
    out
}

/// FormatUtil.replaceFormat: every `&` code as `§`.
fn replace_format(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' && i + 1 < chars.len() && "0123456789abcdefklmnorABCDEFKLMNOR".contains(chars[i + 1]) {
            out.push('§');
            out.push(chars[i + 1].to_ascii_lowercase());
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// FormatUtil.formatString(user, permBase, text): the `&` codes the sender may use become `§` codes
/// (`<base>.color`, `.format`, `.magic`), the others are stripped.
fn format_string(ctx: &mut Ctx, base: &str, s: &str) -> String {
    let color = ctx.has(&format!("{base}.color"));
    let format = ctx.has(&format!("{base}.format"));
    let magic = ctx.has(&format!("{base}.magic"));
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' && i + 1 < chars.len() {
            let c = chars[i + 1].to_ascii_lowercase();
            let allowed = match c {
                '0'..='9' | 'a'..='f' => Some(color),
                'l'..='o' | 'r' => Some(format),
                'k' => Some(magic),
                _ => None,
            };
            if let Some(ok) = allowed {
                if ok {
                    out.push('§');
                    out.push(c);
                }
                i += 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `foo_bar` -> `foo bar`, `\_` -> `_` (item meta values).
fn meta_spaces(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && (chars[i + 1] == '_' || chars[i + 1] == '|') {
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }
        out.push(if chars[i] == '_' { ' ' } else { chars[i] });
        i += 1;
    }
    out
}
/// Splits on `|` not preceded by `\`.
fn split_lines(s: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c == '|' && (i == 0 || chars[i - 1] != '\\') {
            out.push(String::new());
        } else {
            out.last_mut().expect("one").push(*c);
        }
    }
    out
}

/// DateUtil.formatDateDiff from now to `to` (Unix millis).
fn format_date_diff(ess: &Ess, now: i64, to: i64) -> String {
    let diff = (to - now).abs() + 50;
    let mut secs = diff / 1000;
    let units: [(i64, &str, &str); 6] = [(365 * 86400, "year", "years"), (30 * 86400, "month", "months"), (86400, "day", "days"), (3600, "hour", "hours"), (60, "minute", "minutes"), (1, "second", "seconds")];
    let mut parts = Vec::new();
    for (size, one, many) in units {
        if parts.len() > 2 {
            break;
        }
        let n = secs / size;
        if n > 0 {
            secs -= n * size;
            parts.push(format!("{n} {}", ess.message(if n > 1 { many } else { one })));
        }
    }
    if parts.is_empty() { ess.message("now") } else { parts.join(" ") }
}

// ===== Item database (FlatItemDb) =====================================================================

fn index() -> &'static HashMap<&'static str, usize> {
    static INDEX: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map = HashMap::new();
        for (i, (name, _, _, _)) in db::ITEMS.iter().enumerate() {
            map.insert(*name, i);
        }
        for (i, (_, _, _, aliases)) in db::ITEMS.iter().enumerate() {
            for a in aliases.split(' ').filter(|a| !a.is_empty()) {
                map.entry(a).or_insert(i);
            }
        }
        map
    })
}

/// (item path, extra) -> entry, for an item's primary name.
fn reverse_index() -> &'static HashMap<(&'static str, &'static str), usize> {
    static INDEX: OnceLock<HashMap<(&'static str, &'static str), usize>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map = HashMap::new();
        for (i, (_, path, extra, _)) in db::ITEMS.iter().enumerate() {
            map.entry((*path, *extra)).or_insert(i);
        }
        map
    })
}

fn custom_alias(ess: &Ess, name: &str) -> Option<String> {
    let custom = ess.read_yaml("custom_items.yml");
    custom.get("aliases")?.get(name.to_lowercase())?.as_str().map(str::to_string)
}

/// ItemDb.get(name): an item by items.json name or alias, `minecraft:` id, custom_items.yml alias, or any
/// item id the server knows; `name:damage` sets the damage. The count is the item's max stack size.
pub fn item_stack(ess: &mut Ess, host: &mut dyn Host, name: &str) -> Result<Item, CmdError> {
    if let Some(target) = custom_alias(ess, name) {
        return db_get(host, &target);
    }
    db_get(host, name)
}

fn db_get(host: &mut dyn Host, name: &str) -> Result<Item, CmdError> {
    let id = name.to_lowercase();
    let (base, data) = match id.strip_prefix("minecraft:") {
        Some(rest) => match rest.split_once(':') {
            Some((b, d)) => (b.to_string(), Some(d.to_string())),
            None => (rest.to_string(), None),
        },
        None => match id.split_once(':') {
            Some((b, d)) => (b.to_string(), Some(d.to_string())),
            None => (id.clone(), None),
        },
    };
    let ids = host.item_ids();
    let (path, extra) = match index().get(base.as_str()) {
        Some(&i) => (db::ITEMS[i].1.to_string(), db::ITEMS[i].2),
        None if ids.iter().any(|x| *x == ns(&base)) => (base.clone(), ""),
        None => return Err(tl("unknownItemName", vec![plain(id)])),
    };
    let full = ns(&path);
    if !ids.is_empty() && !ids.contains(&full) {
        return Err(tl("unableToSpawnItem", vec![plain(id)]));
    }
    let mut item = Item::new(&full, host.max_stack(&full).max(1));
    if let Some(potion) = extra.strip_prefix("potion:") {
        set_comp(&mut item, "potion_contents", Some(json!({"potion": ns(potion)})));
    }
    if let Some(d) = data {
        let damage: i64 = d.parse().map_err(|_| CmdError::Msg(format!("For input string: \"{d}\"")))?;
        if host.max_damage(&full) > 0 && damage > 0 {
            set_comp(&mut item, "damage", Some(json!(damage)));
        }
    }
    if let Some(entity) = extra.strip_prefix("entity:") {
        set_comp(&mut item, "block_entity_data", Some(json!({"id": "minecraft:mob_spawner", "SpawnData": {"entity": {"id": ns(entity)}}})));
    }
    Ok(item)
}

fn item_extra(item: &Item) -> String {
    if let Some(p) = comp(item, "potion_contents").and_then(|p| p.get("potion")).and_then(Value::as_str) {
        return format!("potion:{}", bare(p));
    }
    if let Some(e) = comp(item, "block_entity_data").and_then(|b| b.pointer("/SpawnData/entity/id")).and_then(Value::as_str) {
        return format!("entity:{}", bare(e));
    }
    String::new()
}

/// ItemDb.name(item): the items.json primary name (`diamond_sword`, `strong_swiftness_potion`).
pub fn item_db_name(item: &Item) -> String {
    let path = bare(&item.id).to_string();
    let extra = item_extra(item);
    let key = (path.as_str(), extra.as_str());
    match reverse_index().iter().find(|((p, e), _)| (*p, *e) == key) {
        Some((_, &i)) => db::ITEMS[i].0.to_string(),
        None => reverse_index().get(&(path.as_str(), "")).map(|&i| db::ITEMS[i].0.to_string()).unwrap_or(path),
    }
}

/// ItemDb.nameList(item): the primary name and every alias.
fn item_names(item: &Item) -> Vec<String> {
    let name = item_db_name(item);
    let mut out = vec![name.clone()];
    if let Some(&i) = index().get(name.as_str()) {
        out.extend(db::ITEMS[i].3.split(' ').filter(|a| !a.is_empty()).map(str::to_string));
    }
    out
}

/// What players see as the item's name in messages (`diamond sword`).
pub fn item_display_name(item: &Item) -> String {
    mat_spaced(item)
}

/// User.canSpawnItem: permission-based-item-spawn, else the item-spawn-blacklist.
fn can_spawn_item(ess: &mut Ess, host: &mut dyn Host, who: Sender, item: &Item) -> bool {
    if who == Sender::Console {
        return true;
    }
    let name = mat_compact(item);
    if ess.config_bool("permission-based-item-spawn", false) {
        return ess.has(host, who, "essentials.itemspawn.item-all") || ess.has(host, who, &format!("essentials.itemspawn.item-{name}"));
    }
    if ess.has(host, who, "essentials.itemspawn.exempt") {
        return true;
    }
    let list = ess.config_str("item-spawn-blacklist").unwrap_or_default();
    for entry in list.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        if let Ok(black) = item_stack(ess, host, entry) {
            if black.id == item.id {
                return false;
            }
        }
    }
    true
}

// ===== Enchantments & potions =========================================================================

/// Enchantments.getByName: a registry id, then EssentialsX's names and aliases. Returns (id, max level).
fn enchantment(host: &mut dyn Host, name: &str) -> Option<(String, i32)> {
    let all = host.enchantments();
    let lower = name.to_lowercase();
    let find = |id: &str| all.iter().find(|(e, _)| bare(e) == bare(id)).cloned();
    if let Some(e) = find(&lower) {
        return Some(e);
    }
    let alias = db::ENCHANTMENTS.iter().rev().find(|(a, _, primary)| *a == lower && *primary).or_else(|| db::ENCHANTMENTS.iter().rev().find(|(a, _, _)| *a == lower))?;
    find(alias.1).or_else(|| all.is_empty().then(|| (ns(alias.1), 1)))
}

/// Potions.getByName: an effect id, then EssentialsX's names and aliases (`speed`, `slow`, `heal`...).
fn potion_effect(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    if VANILLA_EFFECTS.contains(&bare(&lower)) {
        return Some(ns(bare(&lower)));
    }
    let hit = db::POTION_EFFECTS.iter().rev().find(|(a, _, primary)| *a == lower && *primary).or_else(|| db::POTION_EFFECTS.iter().rev().find(|(a, _, _)| *a == lower))?;
    Some(ns(hit.1))
}

const VANILLA_EFFECTS: &[&str] = &[
    "speed", "slowness", "haste", "mining_fatigue", "strength", "instant_health", "instant_damage", "jump_boost", "nausea", "regeneration",
    "resistance", "fire_resistance", "water_breathing", "invisibility", "blindness", "night_vision", "hunger", "weakness", "poison", "wither",
    "health_boost", "absorption", "saturation", "glowing", "levitation", "luck", "unluck", "slow_falling", "conduit_power", "dolphins_grace",
    "bad_omen", "hero_of_the_village", "darkness", "trial_omen", "raid_omen", "wind_charged", "weaving", "oozing", "infested", "breath_of_the_nautilus",
];

/// DyeColor names and getFireworkColor.
const DYE_FIREWORK: [(&str, u32); 16] = [
    ("white", 0xF0F0F0), ("orange", 0xEB8844), ("magenta", 0xC354CD), ("light_blue", 0x6689D3), ("yellow", 0xDECF2A), ("lime", 0x41CD34),
    ("pink", 0xD88198), ("gray", 0x434343), ("light_gray", 0xABABAB), ("cyan", 0x287697), ("purple", 0x7B2FBE), ("blue", 0x253192),
    ("brown", 0x51301A), ("green", 0x3B511A), ("red", 0xB3312C), ("black", 0x1E1B1B),
];
/// DyeColor.getColor (the diffuse colour banners use in EssentialsX's banner meta).
const DYE_COLOR: [(&str, u32); 16] = [
    ("white", 0xF9FFFE), ("orange", 0xF9801D), ("magenta", 0xC74EBD), ("light_blue", 0x3AB3DA), ("yellow", 0xFED83D), ("lime", 0x80C71F),
    ("pink", 0xF38BAA), ("gray", 0x474F52), ("light_gray", 0x9D9D97), ("cyan", 0x169C9C), ("purple", 0x8932B8), ("blue", 0x3C44AA),
    ("brown", 0x835432), ("green", 0x5E7C16), ("red", 0xB02E26), ("black", 0x1D1D21),
];
/// Bukkit PatternType identifiers -> banner pattern ids.
const BANNER_PATTERNS: &[(&str, &str)] = &[
    ("b", "base"), ("bl", "square_bottom_left"), ("br", "square_bottom_right"), ("tl", "square_top_left"), ("tr", "square_top_right"),
    ("bs", "stripe_bottom"), ("ts", "stripe_top"), ("ls", "stripe_left"), ("rs", "stripe_right"), ("cs", "stripe_center"), ("ms", "stripe_middle"),
    ("drs", "stripe_downright"), ("dls", "stripe_downleft"), ("ss", "small_stripes"), ("cr", "cross"), ("sc", "straight_cross"),
    ("bt", "triangle_bottom"), ("tt", "triangle_top"), ("bts", "triangles_bottom"), ("tts", "triangles_top"), ("ld", "diagonal_left"),
    ("rd", "diagonal_up_right"), ("lud", "diagonal_up_left"), ("rud", "diagonal_right"), ("mc", "circle"), ("mr", "rhombus"),
    ("vh", "half_vertical"), ("hh", "half_horizontal"), ("vhr", "half_vertical_right"), ("hhb", "half_horizontal_bottom"), ("bo", "border"),
    ("cbo", "curly_border"), ("cre", "creeper"), ("gra", "gradient"), ("gru", "gradient_up"), ("bri", "bricks"), ("sku", "skull"),
    ("flo", "flower"), ("moj", "mojang"), ("glb", "globe"), ("pig", "piglin"), ("flw", "flow"), ("gus", "guster"),
];
/// ItemFlag -> the components `tooltip_display.hidden_components` hides.
const ITEM_FLAGS: &[(&str, &[&str])] = &[
    ("HIDE_ENCHANTS", &["minecraft:enchantments"]),
    ("HIDE_ATTRIBUTES", &["minecraft:attribute_modifiers"]),
    ("HIDE_UNBREAKABLE", &["minecraft:unbreakable"]),
    ("HIDE_DESTROYS", &["minecraft:can_break"]),
    ("HIDE_PLACED_ON", &["minecraft:can_place_on"]),
    ("HIDE_ADDITIONAL_TOOLTIP", &["minecraft:potion_contents", "minecraft:written_book_content", "minecraft:fireworks", "minecraft:firework_explosion"]),
    ("HIDE_DYE", &["minecraft:dyed_color"]),
    ("HIDE_ARMOR_TRIM", &["minecraft:trim"]),
    ("HIDE_STORED_ENCHANTS", &["minecraft:stored_enchantments"]),
];

fn is_firework(item: &Item) -> bool {
    bare(&item.id) == "firework_rocket"
}
fn is_firework_star(item: &Item) -> bool {
    bare(&item.id) == "firework_star"
}
fn is_potion(item: &Item) -> bool {
    matches!(bare(&item.id), "potion" | "splash_potion" | "lingering_potion" | "tipped_arrow")
}
fn is_banner(item: &Item) -> bool {
    let p = bare(&item.id);
    (p.ends_with("_banner") && !p.ends_with("_wall_banner")) || p == "shield"
}
fn is_leather(item: &Item) -> bool {
    bare(&item.id).starts_with("leather_") && armor_slot(&item.id).is_some()
}
fn is_book(item: &Item) -> bool {
    matches!(bare(&item.id), "written_book" | "writable_book")
}

fn fireworks_of(item: &Item) -> Value {
    let mut f = comp(item, "fireworks").cloned().unwrap_or_else(|| json!({}));
    if f.get("flight_duration").is_none() {
        f["flight_duration"] = json!(1);
    }
    if f.get("explosions").is_none() {
        f["explosions"] = json!([]);
    }
    f
}

// ===== MetaItemStack ===================================================================================

#[derive(Default)]
struct Firework {
    colors: Vec<u32>,
    fade: Vec<u32>,
    shape: Option<&'static str>,
    twinkle: bool,
    trail: bool,
}

impl Firework {
    fn build(&self) -> Result<Value, CmdError> {
        if self.colors.is_empty() {
            return Err(CmdError::Msg("Must have at least one color".into()));
        }
        let mut e = Map::new();
        e.insert("shape".into(), json!(self.shape.unwrap_or("small_ball")));
        e.insert("colors".into(), json!(self.colors));
        if !self.fade.is_empty() {
            e.insert("fade_colors".into(), json!(self.fade));
        }
        if self.trail {
            e.insert("has_trail".into(), json!(true));
        }
        if self.twinkle {
            e.insert("has_twinkle".into(), json!(true));
        }
        Ok(Value::Object(e))
    }
}

/// MetaItemStack: an item plus the meta parsed so far (fireworks and potions are built over several args).
struct Meta {
    item: Item,
    firework: Firework,
    valid_firework: bool,
    valid_charge: bool,
    effect: Option<String>,
    valid_effect: bool,
    valid_duration: bool,
    valid_power: bool,
    splash: bool,
    complete_potion: bool,
    power: i32,
    duration: i32,
}

/// Who parses meta: None for kits (no permission checks), else the sender.
type MetaSender = Option<Sender>;

fn split_meta(s: &str) -> Vec<String> {
    match s.find([':', '+', '\'', ',', ';', '.']) {
        Some(i) => vec![s[..i].to_string(), s[i + 1..].to_string()],
        None => vec![s.to_string()],
    }
}

fn parse_color(c: &str) -> Option<u32> {
    if let Some((_, rgb)) = DYE_FIREWORK.iter().find(|(n, _)| n.eq_ignore_ascii_case(c)) {
        return Some(*rgb);
    }
    let hex = c.strip_prefix('#')?;
    (hex.len() == 6).then(|| u32::from_str_radix(hex, 16).ok()).flatten()
}

fn firework_shape(s: &str) -> Option<&'static str> {
    Some(match s.to_ascii_uppercase().as_str() {
        "BALL" => "small_ball",
        "BALL_LARGE" | "LARGE" => "large_ball",
        "STAR" => "star",
        "BURST" => "burst",
        "CREEPER" => "creeper",
        _ => return None,
    })
}

impl Meta {
    fn new(item: Item) -> Self {
        Meta {
            item,
            firework: Firework::default(),
            valid_firework: false,
            valid_charge: false,
            effect: None,
            valid_effect: false,
            valid_duration: false,
            valid_power: false,
            splash: false,
            complete_potion: false,
            power: 1,
            duration: 120,
        }
    }

    /// hasMetaPermission: `essentials.itemspawn.meta-<x>` (include_base) or `essentials.<x>`; graceful
    /// answers false instead of the noMetaPerm error.
    fn perm(ess: &mut Ess, host: &mut dyn Host, sender: MetaSender, node: &str, graceful: bool, include_base: bool) -> Result<bool, CmdError> {
        let Some(who @ Sender::Player(_)) = sender else { return Ok(true) };
        let full = if include_base { format!("essentials.itemspawn.meta-{node}") } else { format!("essentials.{node}") };
        if ess.has(host, who, &full) {
            return Ok(true);
        }
        if graceful { Ok(false) } else { Err(tl("noMetaPerm", vec![plain(node)])) }
    }

    /// parseStringMeta: every arg from `from`, then the firework built from them.
    fn parse_all(&mut self, ess: &mut Ess, host: &mut dyn Host, sender: MetaSender, unsafe_: bool, args: &[String], from: usize) -> Result<(), CmdError> {
        let Some(first) = args.get(from) else { return Ok(()) };
        if first.starts_with('{') && Self::perm(ess, host, sender, "vanilla", false, true)? {
            return Err(tl("noMetaNbtKill", vec![]));
        }
        if first.starts_with('[') && Self::perm(ess, host, sender, "vanilla", false, true)? {
            let text = final_arg(args, from);
            let parsed = parse_component_list(&text).ok_or_else(|| CmdError::Msg(format!("Malformed item components: {text}")))?;
            for (k, v) in parsed {
                set_comp(&mut self.item, bare(&k), Some(v));
            }
            return Ok(());
        }
        for arg in &args[from..] {
            self.add_meta(ess, host, sender, unsafe_, arg)?;
        }
        if self.valid_firework {
            if !Self::perm(ess, host, sender, "firework", true, true)? {
                return Err(tl("noMetaFirework", vec![]));
            }
            let effect = self.firework.build()?;
            let mut f = fireworks_of(&self.item);
            f["explosions"].as_array_mut().expect("a list").push(effect);
            let n = f["explosions"].as_array().map_or(0, Vec::len);
            if n > 1 && !Self::perm(ess, host, sender, "firework-multiple", true, true)? {
                return Err(tl("multipleCharges", vec![]));
            }
            set_comp(&mut self.item, "fireworks", Some(f));
        }
        if self.valid_charge {
            if !Self::perm(ess, host, sender, "firework", true, true)? {
                return Err(tl("noMetaFirework", vec![]));
            }
            let effect = self.firework.build()?;
            set_comp(&mut self.item, "firework_explosion", Some(effect));
        }
        Ok(())
    }

    /// addStringMeta: one `key:value` (or enchantment) argument.
    fn add_meta(&mut self, ess: &mut Ess, host: &mut dyn Host, sender: MetaSender, unsafe_: bool, s: &str) -> Result<(), CmdError> {
        let split = split_meta(s);
        let key = split[0].to_lowercase();
        let value = split.get(1).cloned();
        let has_value = value.is_some();
        let v = value.clone().unwrap_or_default();
        if has_value && key == "name" && Self::perm(ess, host, sender, "name", false, true)? {
            let name = replace_format(&meta_spaces(&v));
            set_comp(&mut self.item, "custom_name", Some(legacy_component(&name)));
        } else if has_value && (key == "lore" || key == "desc") && Self::perm(ess, host, sender, "lore", false, true)? {
            let lore: Vec<Value> = split_lines(&v).iter().map(|l| legacy_component(&replace_format(&meta_spaces(l)))).collect();
            set_comp(&mut self.item, "lore", Some(json!(lore)));
        } else if (key == "custom-model-data" || key == "cmd") && Self::perm(ess, host, sender, "custom-model-data", false, true)? {
            let n: i32 = if has_value { v.parse().map_err(|_| CmdError::Msg(format!("For input string: \"{v}\"")))? } else { 0 };
            set_comp(&mut self.item, "custom_model_data", Some(json!({"floats": [n as f64]})));
        } else if key == "unbreakable" && Self::perm(ess, host, sender, "unbreakable", false, true)? {
            let on = !has_value || v.eq_ignore_ascii_case("true");
            set_comp(&mut self.item, "unbreakable", on.then(|| json!({})));
        } else if has_value && (key == "player" || key == "owner") && Self::perm(ess, host, sender, "head", false, true)? {
            if bare(&self.item.id) != "player_head" {
                return Err(tl("onlyPlayerSkulls", vec![]));
            }
            set_comp(&mut self.item, "profile", Some(json!({"name": v})));
        } else if has_value
            && key == "book"
            && is_book(&self.item)
            && (Self::perm(ess, host, sender, "book", true, true)? || Self::perm(ess, host, sender, &format!("chapter-{}", v.to_lowercase()), true, true)?)
        {
            let pages = book_pages(ess, &v);
            if bare(&self.item.id) == "written_book" {
                let mut content = comp(&self.item, "written_book_content").cloned().unwrap_or_else(|| json!({}));
                if content.get("author").is_none() {
                    let author = match sender {
                        Some(Sender::Player(p)) => host.name(p).unwrap_or_default(),
                        _ => "Console".to_string(),
                    };
                    content["author"] = json!(author);
                }
                if content.get("title").is_none() {
                    let title: String = replace_format(&v.replace('_', " ")).chars().take(32).collect();
                    content["title"] = json!({"raw": title});
                }
                content["pages"] = json!(pages.iter().map(|p| json!({"raw": p})).collect::<Vec<_>>());
                set_comp(&mut self.item, "written_book_content", Some(content));
            } else {
                set_comp(&mut self.item, "writable_book_content", Some(json!({"pages": pages.iter().map(|p| json!({"raw": p})).collect::<Vec<_>>()})));
            }
        } else if has_value && key == "author" && bare(&self.item.id) == "written_book" && Self::perm(ess, host, sender, "author", false, true)? {
            let mut content = comp(&self.item, "written_book_content").cloned().unwrap_or_else(|| json!({"title": {"raw": ""}}));
            content["author"] = json!(replace_format(&v));
            set_comp(&mut self.item, "written_book_content", Some(content));
        } else if has_value && key == "title" && bare(&self.item.id) == "written_book" && Self::perm(ess, host, sender, "title", false, true)? {
            let mut content = comp(&self.item, "written_book_content").cloned().unwrap_or_else(|| json!({"author": ""}));
            content["title"] = json!({"raw": replace_format(&meta_spaces(&v))});
            set_comp(&mut self.item, "written_book_content", Some(content));
        } else if has_value && key.starts_with("page") && key.len() > 4 && is_book(&self.item) && Self::perm(ess, host, sender, "page", false, true)? {
            let page = key[4..].parse::<i32>().map(|n| n - 1).unwrap_or(0);
            if page > 100 {
                return Err(tl("pageLimitExceeded", vec![]));
            }
            let content = split_lines(&v).iter().map(|l| replace_format(&meta_spaces(l))).collect::<Vec<_>>().join("\n");
            let written = bare(&self.item.id) == "written_book";
            let key_c = if written { "written_book_content" } else { "writable_book_content" };
            let mut c = comp(&self.item, key_c).cloned().unwrap_or_else(|| if written { json!({"title": {"raw": ""}, "author": ""}) } else { json!({}) });
            let mut pages = c.get("pages").and_then(Value::as_array).cloned().unwrap_or_default();
            let page = page.max(0) as usize;
            while pages.len() <= page {
                pages.push(json!({"raw": ""}));
            }
            pages[page] = json!({"raw": content});
            c["pages"] = json!(pages);
            set_comp(&mut self.item, key_c, Some(c));
        } else if has_value && key == "power" && is_firework(&self.item) && Self::perm(ess, host, sender, "firework-power", false, true)? {
            let power = v.parse::<i32>().unwrap_or(0);
            let mut f = fireworks_of(&self.item);
            f["flight_duration"] = json!(if power > 3 { 4 } else { power });
            set_comp(&mut self.item, "fireworks", Some(f));
        } else if has_value && key == "itemflags" && Self::perm(ess, host, sender, "itemflags", false, true)? {
            self.add_item_flags(s)?;
        } else if is_firework(&self.item) {
            if !self.enchantment_meta(ess, host, sender, unsafe_, &split)? {
                self.firework_meta(ess, host, sender, false, s)?;
            }
        } else if is_firework_star(&self.item) {
            if !self.enchantment_meta(ess, host, sender, unsafe_, &split)? {
                self.charge_meta(false, s)?;
            }
        } else if is_potion(&self.item) {
            if key == "power" || !self.enchantment_meta(ess, host, sender, unsafe_, &split)? {
                self.potion_meta(ess, host, sender, false, s)?;
            }
        } else if is_banner(&self.item) {
            if !self.enchantment_meta(ess, host, sender, unsafe_, &split)? {
                self.banner_meta(s)?;
            }
        } else if has_value && (key == "color" || key == "colour") && is_leather(&self.item) {
            let parts: Vec<&str> = v.split(['|', ',']).collect();
            let rgb = if parts.len() == 1 && (is_int(parts[0]) || parts[0].starts_with('#')) {
                if let Some(hex) = parts[0].strip_prefix('#') {
                    i64::from_str_radix(hex.get(..6).unwrap_or(hex), 16).map_err(|_| tl("leatherSyntax", vec![]))?
                } else {
                    parts[0].parse::<i64>().unwrap_or(0)
                }
            } else if parts.len() == 3 {
                let c = |s: &str| s.parse::<i64>().unwrap_or(0).clamp(0, 255);
                (c(parts[0]) << 16) | (c(parts[1]) << 8) | c(parts[2])
            } else {
                return Err(tl("leatherSyntax", vec![]));
            };
            set_comp(&mut self.item, "dyed_color", Some(json!(rgb)));
        } else if armor_slot(&self.item.id).is_some() && has_value && key == "trim" {
            let (pattern, material) = v.split_once('|').ok_or_else(|| CmdError::Msg(format!("Invalid trim: {v}")))?;
            set_comp(&mut self.item, "trim", Some(json!({"material": ns(&material.to_lowercase()), "pattern": ns(&pattern.to_lowercase())})));
        } else {
            self.enchantment_meta(ess, host, sender, unsafe_, &split)?;
        }
        Ok(())
    }

    fn add_item_flags(&mut self, s: &str) -> Result<(), CmdError> {
        let split = split_meta(s);
        let Some(list) = split.get(1) else { return Err(tl("invalidItemFlagMeta", vec![plain(s)])) };
        let mut td = comp(&self.item, "tooltip_display").cloned().unwrap_or_else(|| json!({}));
        let mut hidden: Vec<Value> = td.get("hidden_components").and_then(Value::as_array).cloned().unwrap_or_default();
        for f in list.split(',') {
            if let Some((_, ids)) = ITEM_FLAGS.iter().find(|(n, _)| n.eq_ignore_ascii_case(f)) {
                for id in *ids {
                    if !hidden.contains(&json!(id)) {
                        hidden.push(json!(id));
                    }
                }
            }
        }
        if hidden.is_empty() {
            return Err(tl("invalidItemFlagMeta", vec![plain(s)]));
        }
        td["hidden_components"] = json!(hidden);
        set_comp(&mut self.item, "tooltip_display", Some(td));
        Ok(())
    }

    fn colors(list: &str, split: &[String]) -> Result<Vec<u32>, CmdError> {
        list.split(',').map(|c| parse_color(c).ok_or_else(|| tl("invalidFireworkFormat", vec![plain(&split[1]), plain(&split[0])]))).collect()
    }

    /// addChargeMeta (firework stars).
    fn charge_meta(&mut self, short: bool, s: &str) -> Result<(), CmdError> {
        let split = split_meta(s);
        if split.len() < 2 {
            return Ok(());
        }
        let k = split[0].to_lowercase();
        if k == "color" || k == "colour" || (short && k == "c") {
            self.firework.colors = Self::colors(&split[1], &split)?;
            self.valid_charge = true;
        } else if k == "shape" || k == "type" || (short && (k == "s" || k == "t")) {
            self.firework.shape = Some(firework_shape(&split[1]).ok_or_else(|| tl("invalidFireworkFormat", vec![plain(&split[1]), plain(&split[0])]))?);
        } else if k == "fade" || (short && k == "f") {
            self.firework.fade = Self::colors(&split[1], &split)?;
        } else if k == "effect" || (short && k == "e") {
            self.firework_effects(&split)?;
        }
        Ok(())
    }

    fn firework_effects(&mut self, split: &[String]) -> Result<(), CmdError> {
        for e in split[1].split(',') {
            if e.eq_ignore_ascii_case("twinkle") {
                self.firework.twinkle = true;
            } else if e.eq_ignore_ascii_case("trail") {
                self.firework.trail = true;
            } else {
                return Err(tl("invalidFireworkFormat", vec![plain(&split[1]), plain(&split[0])]));
            }
        }
        Ok(())
    }

    /// addFireworkMeta (rockets): a new `color:` starts a new explosion.
    fn firework_meta(&mut self, ess: &mut Ess, host: &mut dyn Host, sender: MetaSender, short: bool, s: &str) -> Result<(), CmdError> {
        let split = split_meta(s);
        if split.len() < 2 {
            return Ok(());
        }
        let k = split[0].to_lowercase();
        if k == "color" || k == "colour" || (short && k == "c") {
            if self.valid_firework {
                if !Self::perm(ess, host, sender, "firework", true, true)? {
                    return Err(tl("noMetaFirework", vec![]));
                }
                let effect = self.firework.build()?;
                let mut f = fireworks_of(&self.item);
                f["explosions"].as_array_mut().expect("a list").push(effect);
                if f["explosions"].as_array().map_or(0, Vec::len) > 1 && !Self::perm(ess, host, sender, "firework-multiple", true, true)? {
                    return Err(tl("multipleCharges", vec![]));
                }
                set_comp(&mut self.item, "fireworks", Some(f));
                self.firework = Firework::default();
            }
            self.firework.colors = Self::colors(&split[1], &split)?;
            self.valid_firework = true;
        } else if k == "shape" || k == "type" || (short && (k == "s" || k == "t")) {
            self.firework.shape = Some(firework_shape(&split[1]).ok_or_else(|| tl("invalidFireworkFormat", vec![plain(&split[1]), plain(&split[0])]))?);
        } else if k == "fade" || (short && k == "f") {
            self.firework.fade = Self::colors(&split[1], &split)?;
        } else if k == "effect" || (short && k == "e") {
            self.firework_effects(&split)?;
        }
        Ok(())
    }

    /// addPotionMeta: effect, power/amplifier, duration (seconds), splash; complete once all three are set.
    fn potion_meta(&mut self, ess: &mut Ess, host: &mut dyn Host, sender: MetaSender, short: bool, s: &str) -> Result<(), CmdError> {
        let split = split_meta(s);
        if split.len() < 2 {
            return Ok(());
        }
        let k = split[0].to_lowercase();
        let v = &split[1];
        if k == "effect" || (short && k == "e") {
            let effect = potion_effect(v).ok_or_else(|| tl("invalidPotionMeta", vec![plain(v)]))?;
            if Self::perm(ess, host, sender, &format!("potions.{}", bare(&effect)), true, false)? {
                self.effect = Some(effect);
                self.valid_effect = true;
            } else {
                return Err(tl("noPotionEffectPerm", vec![plain(bare(&effect))]));
            }
        } else if k == "power" || (short && k == "p") {
            let mut p: i32 = v.parse().map_err(|_| tl("invalidPotionMeta", vec![plain(v)]))?;
            if p > 0 && p < 4 {
                p -= 1;
            }
            self.power = p;
            self.valid_power = true;
        } else if k == "amplifier" || (short && k == "a") {
            self.power = v.parse().map_err(|_| tl("invalidPotionMeta", vec![plain(v)]))?;
            self.valid_power = true;
        } else if k == "duration" || (short && k == "d") {
            let d: i32 = v.parse().map_err(|_| tl("invalidPotionMeta", vec![plain(v)]))?;
            self.duration = if d == -1 { -1 } else { d * 20 };
            self.valid_duration = true;
        } else if k == "splash" || (short && k == "s") {
            self.splash = v.eq_ignore_ascii_case("true");
        }
        if self.valid_effect && self.valid_duration && self.valid_power {
            let mut pc = comp(&self.item, "potion_contents").cloned().unwrap_or_else(|| json!({}));
            let mut effects = pc.get("custom_effects").and_then(Value::as_array).cloned().unwrap_or_default();
            if effects.len() > 1 && !Self::perm(ess, host, sender, "potions.multiple", true, false)? {
                return Err(tl("multiplePotionEffects", vec![]));
            }
            let id = self.effect.clone().unwrap_or_default();
            effects.retain(|e| e.get("id").and_then(Value::as_str) != Some(id.as_str()));
            effects.push(json!({"id": id, "amplifier": self.power, "duration": self.duration}));
            pc["custom_effects"] = json!(effects);
            set_comp(&mut self.item, "potion_contents", Some(pc));
            let p = bare(&self.item.id);
            if self.splash && p == "potion" {
                self.item.id = "minecraft:splash_potion".into();
            } else if !self.splash && p == "splash_potion" {
                self.item.id = "minecraft:potion".into();
            }
            self.effect = None;
            self.valid_effect = false;
            self.valid_duration = false;
            self.valid_power = false;
            self.splash = false;
            self.complete_potion = true;
        }
        Ok(())
    }

    /// addBannerMeta: `basecolor:<rgb>` and `<pattern>:<rgb>` (DyeColor.getColor values).
    fn banner_meta(&mut self, s: &str) -> Result<(), CmdError> {
        let split = split_meta(s);
        if split.len() < 2 {
            return Err(tl("invalidBanner", vec![plain(s)]));
        }
        let dye = |v: &str| -> Option<&'static str> {
            let rgb: u32 = v.parse().ok()?;
            DYE_COLOR.iter().find(|(_, c)| *c == rgb).map(|(n, _)| *n)
        };
        let k = split[0].to_lowercase();
        if k == "basecolor" {
            let Some(color) = dye(&split[1]) else { return Ok(()) };
            if bare(&self.item.id) == "shield" {
                set_comp(&mut self.item, "base_color", Some(json!(color)));
            } else {
                self.item.id = format!("minecraft:{color}_banner");
            }
        } else if let Some((_, pattern)) = BANNER_PATTERNS.iter().find(|(code, id)| *code == k || *id == k) {
            let Some(color) = dye(&split[1]) else { return Ok(()) };
            let mut layers = comp(&self.item, "banner_patterns").and_then(Value::as_array).cloned().unwrap_or_default();
            layers.push(json!({"pattern": ns(pattern), "color": color}));
            set_comp(&mut self.item, "banner_patterns", Some(json!(layers)));
        }
        Ok(())
    }

    /// parseEnchantmentStrings: false when the key isn't an enchantment.
    fn enchantment_meta(&mut self, ess: &mut Ess, host: &mut dyn Host, sender: MetaSender, unsafe_: bool, split: &[String]) -> Result<bool, CmdError> {
        let Some((id, max)) = enchantment(host, &split[0]) else { return Ok(false) };
        if Self::perm(ess, host, sender, &format!("enchantments.{}", bare(&id)), false, false)? {
            let mut level = split.get(1).and_then(|l| l.parse::<i32>().ok()).unwrap_or(-1);
            if level < 0 || (!unsafe_ && level > max) {
                level = max;
            }
            add_enchantment(host, &mut self.item, &id, max, level, unsafe_)?;
        }
        Ok(true)
    }
}

/// MetaItemStack.addEnchantment (ItemStack.addEnchantment's checks when not unsafe).
fn add_enchantment(host: &mut dyn Host, item: &mut Item, id: &str, max: i32, level: i32, unsafe_: bool) -> Result<(), CmdError> {
    let wrap = |m: String| CmdError::Msg(format!("Enchantment {}: {m}", bare(id)));
    let stored = bare(&item.id) == "enchanted_book";
    let key = if stored { "stored_enchantments" } else { "enchantments" };
    let mut map = comp(item, key).and_then(Value::as_object).cloned().unwrap_or_default();
    if level == 0 {
        map.remove(id);
    } else if stored {
        if !unsafe_ && (level < 1 || level > max) {
            return Ok(());
        }
        map.insert(id.to_string(), json!(level));
    } else {
        if !unsafe_ {
            if level < 1 || level > max {
                return Err(wrap(format!("Enchantment level is either too low or too high (given {level}, bounds are 1 to {max})")));
            }
            if !host.can_enchant(&item.id, id) {
                return Err(wrap("Specified enchantment cannot be applied to this itemstack".into()));
            }
        }
        map.insert(id.to_string(), json!(level));
    }
    set_comp(item, key, (!map.is_empty()).then(|| Value::Object(map)));
    Ok(())
}

/// `[id=value,...]` data components as typed by players (vanilla's item argument), SNBT-ish values as JSON.
fn parse_component_list(text: &str) -> Option<Vec<(String, Value)>> {
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    let mut out = Vec::new();
    for part in split_top(inner, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (k, v) = part.split_once('=')?;
        out.push((k.trim().to_string(), snbt_value(v.trim())?));
    }
    Some(out)
}

fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let (mut depth, mut quote) = (0i32, None::<char>);
    for c in s.chars() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '[' | '{') => depth += 1,
            (None, ']' | '}') => depth -= 1,
            (None, _) if c == sep && depth == 0 => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().expect("one").push(c);
    }
    out
}

/// A small SNBT reader for component values typed in commands.
fn snbt_value(s: &str) -> Option<Value> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        let mut m = Map::new();
        for part in split_top(inner, ',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (k, v) = part.split_once(':')?;
            m.insert(k.trim().trim_matches('"').trim_matches('\'').to_string(), snbt_value(v)?);
        }
        return Some(Value::Object(m));
    }
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let inner = inner.strip_prefix("I;").unwrap_or(inner);
        return split_top(inner, ',').iter().filter(|p| !p.trim().is_empty()).map(|p| snbt_value(p)).collect::<Option<Vec<_>>>().map(Value::Array);
    }
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2) || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2) {
        return Some(json!(s[1..s.len() - 1]));
    }
    match s {
        "true" => return Some(json!(true)),
        "false" => return Some(json!(false)),
        _ => {}
    }
    let num = s.trim_end_matches(['b', 'B', 's', 'S', 'l', 'L', 'f', 'F', 'd', 'D']);
    if let Ok(i) = num.parse::<i64>() {
        return Some(json!(i));
    }
    if let Ok(f) = num.parse::<f64>() {
        return Some(json!(f));
    }
    Some(json!(s))
}

/// JSON as SNBT (entity data for spawned fireworks).
fn json_to_snbt(v: &Value) -> String {
    match v {
        Value::Null => "{}".into(),
        Value::Bool(b) => if *b { "1b" } else { "0b" }.into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Array(a) => format!("[{}]", a.iter().map(json_to_snbt).collect::<Vec<_>>().join(",")),
        Value::Object(o) => format!("{{{}}}", o.iter().map(|(k, v)| format!("\"{k}\":{}", json_to_snbt(v))).collect::<Vec<_>>().join(",")),
    }
}

/// book.txt chapters (`#name` headings), paged like BookPager: 14 lines a page.
fn book_pages(ess: &Ess, chapter: &str) -> Vec<String> {
    let text = std::fs::read_to_string(ess.dir.join("book.txt")).unwrap_or_default();
    let mut lines = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix('#') {
            if inside {
                break;
            }
            inside = name.trim().eq_ignore_ascii_case(chapter);
            continue;
        }
        if inside {
            lines.push(replace_format(line));
        }
    }
    if lines.is_empty() {
        lines = text.lines().take_while(|l| !l.starts_with('#')).map(replace_format).collect();
    }
    lines.chunks(14).map(|c| c.join("\n")).collect()
}

/// ItemDb.serialize: the kit line for an item (`diamond_sword 1 name:Blade sharpness:2`).
pub fn serialize_item(item: &Item) -> String {
    let mut sb: Vec<String> = vec![item_db_name(item), item.count.to_string()];
    let unformat = |c: &Value| component_legacy(c);
    let lines = |v: &[Value]| v.iter().map(|l| unformat(l).replace(' ', "_").replace('|', "\\|")).collect::<Vec<_>>().join("|");
    if let Some(name) = comp(item, "custom_name") {
        sb.push(format!("name:{}", unformat(name).replace(' ', "_")));
    }
    if let Some(Value::Array(lore)) = comp(item, "lore") {
        sb.push(format!("lore:{}", lines(lore)));
    }
    if let Some(n) = comp(item, "custom_model_data").and_then(|c| c.pointer("/floats/0")).and_then(Value::as_f64) {
        sb.push(format!("custom-model-data:{}", n as i64));
    }
    for (e, l) in enchants_of(item) {
        sb.push(format!("{}:{}", bare(&e), l));
    }
    if let Some(Value::Array(hidden)) = comp(item, "tooltip_display").and_then(|t| t.get("hidden_components")) {
        let flags: Vec<&str> = ITEM_FLAGS.iter().filter(|(_, ids)| ids.iter().any(|id| hidden.contains(&json!(id)))).map(|(n, _)| *n).collect();
        if !flags.is_empty() {
            sb.push(format!("itemflags:{}", flags.join(",")));
        }
    }
    match bare(&item.id) {
        "written_book" | "writable_book" => {
            let key = if bare(&item.id) == "written_book" { "written_book_content" } else { "writable_book_content" };
            if let Some(c) = comp(item, key) {
                if let Some(t) = c.get("title") {
                    sb.push(format!("title:{}", unformat(t).replace(' ', "_")));
                }
                if let Some(a) = c.get("author").and_then(Value::as_str) {
                    sb.push(format!("author:{}", a.replace(' ', "_")));
                }
                if let Some(Value::Array(pages)) = c.get("pages") {
                    for (i, p) in pages.iter().enumerate() {
                        let text = component_plain(p);
                        let page: Vec<Value> = text.split('\n').map(|l| json!(l)).collect();
                        sb.push(format!("page{}:{}", i + 1, lines(&page)));
                    }
                }
            }
        }
        "enchanted_book" => {
            if let Some(Value::Object(m)) = comp(item, "stored_enchantments") {
                for (e, l) in m {
                    sb.push(format!("{}:{}", bare(e), l));
                }
            }
        }
        _ => {}
    }
    let effect = |sb: &mut Vec<String>, e: &Value| {
        let hex = |c: &Value| format!("#{:x}", c.as_u64().unwrap_or(0));
        if let Some(Value::Array(c)) = e.get("colors") {
            sb.push(format!("color:{}", c.iter().map(hex).collect::<Vec<_>>().join(",")));
        }
        let shape = match e.get("shape").and_then(Value::as_str).unwrap_or("small_ball") {
            "large_ball" => "BALL_LARGE",
            "star" => "STAR",
            "burst" => "BURST",
            "creeper" => "CREEPER",
            _ => "BALL",
        };
        sb.push(format!("shape:{shape}"));
        if let Some(Value::Array(c)) = e.get("fade_colors") {
            sb.push(format!("fade:{}", c.iter().map(hex).collect::<Vec<_>>().join(",")));
        }
    };
    if is_firework(item) {
        let f = fireworks_of(item);
        if let Some(Value::Array(ex)) = f.get("explosions").filter(|e| e.as_array().is_some_and(|a| !a.is_empty())) {
            for e in ex {
                effect(&mut sb, e);
            }
            sb.push(format!("power:{}", f["flight_duration"]));
        }
    } else if is_firework_star(item) {
        if let Some(e) = comp(item, "firework_explosion") {
            effect(&mut sb, e);
        }
    } else if is_potion(item) {
        let splash = bare(&item.id) == "splash_potion";
        if let Some(Value::Array(effects)) = comp(item, "potion_contents").and_then(|p| p.get("custom_effects")) {
            for e in effects {
                let id = e.get("id").and_then(Value::as_str).unwrap_or("");
                let amp = e.get("amplifier").and_then(Value::as_i64).unwrap_or(0);
                let dur = e.get("duration").and_then(Value::as_i64).unwrap_or(0);
                sb.push(format!("splash:{splash} effect:{} power:{amp} duration:{}", bare(id), dur / 20));
            }
        }
    } else if bare(&item.id) == "player_head" {
        if let Some(name) = comp(item, "profile").and_then(|p| p.get("name").or(Some(p))).and_then(Value::as_str) {
            sb.push(format!("player:{name}"));
        }
    } else if is_banner(item) {
        let base = if bare(&item.id) == "shield" {
            comp(item, "base_color").and_then(Value::as_str).map(str::to_string)
        } else {
            bare(&item.id).strip_suffix("_banner").map(str::to_string)
        };
        if let Some(rgb) = base.and_then(|b| DYE_COLOR.iter().find(|(n, _)| *n == b).map(|(_, c)| *c)) {
            sb.push(format!("basecolor:{rgb}"));
        }
        if let Some(Value::Array(layers)) = comp(item, "banner_patterns") {
            for l in layers {
                let pattern = l.get("pattern").and_then(Value::as_str).map(bare).unwrap_or("");
                let code = BANNER_PATTERNS.iter().find(|(_, id)| *id == pattern).map(|(c, _)| *c).unwrap_or(pattern);
                let color = l.get("color").and_then(Value::as_str).unwrap_or("white");
                let rgb = DYE_COLOR.iter().find(|(n, _)| *n == color).map(|(_, c)| *c).unwrap_or(0);
                sb.push(format!("{code},{rgb}"));
            }
        }
    } else if armor_slot(&item.id).is_some() {
        if let Some(t) = comp(item, "trim") {
            let g = |k: &str| t.get(k).and_then(Value::as_str).map(bare).unwrap_or("").to_string();
            sb.push(format!("trim:{}|{}", g("pattern"), g("material")));
        }
        if is_leather(item) {
            if let Some(c) = comp(item, "dyed_color").and_then(|c| c.as_i64().or_else(|| c.get("rgb").and_then(Value::as_i64))) {
                sb.push(format!("color:{c}"));
            }
        }
    }
    sb.join(" ").trim().replace('§', "&")
}

/// One kits.yml / `/give`-style line without the leading `slot:`: `<item> [amount] [meta...]`.
/// `sender` None skips permission checks (kits).
pub fn parse_item_line(ess: &mut Ess, host: &mut dyn Host, sender: Option<Sender>, line: &str, allow_unsafe: bool) -> Result<Option<Item>, CmdError> {
    let parts: Vec<String> = line.split(' ').filter(|p| !p.is_empty()).map(str::to_string).collect();
    let Some(name) = parts.first() else { return Ok(None) };
    let mut item = item_stack(ess, host, name)?;
    item.count = match parts.get(1) {
        Some(n) => n.parse().map_err(|_| CmdError::Msg(format!("For input string: \"{n}\"")))?,
        None => 1,
    };
    if bare(&item.id) == "air" {
        return Ok(None);
    }
    if parts.len() > 2 {
        let mut meta = Meta::new(item);
        meta.parse_all(ess, host, sender, allow_unsafe, &parts, 2)?;
        item = meta.item;
    }
    Ok(Some(item))
}

// ===== Inventories (craftbukkit/Inventories) ===============================================================

const BOOTS: usize = 36;
const LEGS: usize = 37;
const CHEST: usize = 38;
const HELM: usize = 39;
const OFFHAND: usize = 40;

fn armor_slot(id: &str) -> Option<usize> {
    let p = bare(id);
    if p.ends_with("_helmet") {
        Some(HELM)
    } else if p.ends_with("_chestplate") {
        Some(CHEST)
    } else if p.ends_with("_leggings") {
        Some(LEGS)
    } else if p.ends_with("_boots") {
        Some(BOOTS)
    } else {
        None
    }
}
fn is_armor_slot(slot: usize) -> bool {
    (BOOTS..=HELM).contains(&slot)
}

/// A player's 41 slots, written back with `save` (only the slots that changed).
struct Inv {
    player: Uuid,
    slots: Vec<Option<Item>>,
    before: Vec<Option<Item>>,
}

impl Inv {
    fn of(host: &mut dyn Host, player: Uuid) -> Inv {
        let mut slots = host.inventory(player);
        slots.resize(41, None);
        for s in slots.iter_mut() {
            if s.as_ref().is_some_and(|i| i.count <= 0 || bare(&i.id) == "air") {
                *s = None;
            }
        }
        Inv { player, before: slots.clone(), slots }
    }
    fn save(self, host: &mut dyn Host) {
        for (i, (now, was)) in self.slots.iter().zip(self.before.iter()).enumerate() {
            if now != was {
                host.set_slot(self.player, i, now.clone());
            }
        }
    }
    fn get(&self, slot: usize) -> Option<&Item> {
        self.slots.get(slot).and_then(Option::as_ref)
    }
}

fn item_max(host: &mut dyn Host, item: &Item, max_stack: i32) -> i32 {
    max_stack.max(host.max_stack(&item.id).max(1))
}

/// normalizeItems: similar stacks merged.
fn normalize(items: Vec<Item>) -> Vec<Item> {
    if items.len() <= 1 {
        return items;
    }
    let mut out: Vec<Item> = Vec::new();
    for it in items.into_iter().filter(|i| i.count > 0) {
        match out.iter_mut().find(|o| similar(o, &it)) {
            Some(o) => o.count += it.count,
            None => out.push(it),
        }
    }
    out
}

/// parseInventoryData: the empty slots and, per item, the partial slots it can fill.
fn inventory_data(host: &mut dyn Host, inv: &Inv, items: &[Item], max_stack: i32, include_armor: bool) -> (Vec<usize>, Vec<Vec<usize>>) {
    let mut empty = Vec::new();
    let mut partial = vec![Vec::new(); items.len()];
    for i in 0..41 {
        if !include_armor && is_armor_slot(i) {
            continue;
        }
        match inv.get(i) {
            None => empty.push(i),
            Some(have) => {
                for (k, new) in items.iter().enumerate() {
                    if have.count < item_max(host, have, max_stack) && similar(have, new) {
                        partial[k].push(i);
                    }
                }
            }
        }
    }
    if include_armor {
        let mut seen = [false; 4];
        for (k, it) in items.iter().enumerate() {
            if let Some(slot) = armor_slot(&it.id) {
                let n = slot - BOOTS;
                if !seen[n] {
                    seen[n] = true;
                    if empty.contains(&slot) {
                        partial[k].push(slot);
                    }
                }
            }
        }
        empty.retain(|s| !is_armor_slot(*s));
    }
    (empty, partial)
}

/// Inventories.hasSpace.
fn has_space(host: &mut dyn Host, inv: &Inv, max_stack: i32, include_armor: bool, items: &[Item]) -> bool {
    let items = normalize(items.to_vec());
    let (mut empty, mut partial) = inventory_data(host, inv, &items, max_stack, include_armor);
    for (k, item) in items.iter().enumerate() {
        let mut amount = item.count;
        let max = item_max(host, item, max_stack);
        loop {
            if partial[k].is_empty() {
                if empty.is_empty() {
                    return false;
                }
                empty.remove(0);
                if amount > max {
                    amount -= max;
                } else {
                    break;
                }
            } else {
                let slot = partial[k].remove(0);
                let existing = inv.get(slot).map_or(0, |i| i.count);
                if amount + existing <= max {
                    break;
                }
                amount = amount + existing - max;
            }
        }
    }
    true
}

/// Inventories.addItem: fills partial stacks, then empty slots; returns what didn't fit.
fn add_to(host: &mut dyn Host, inv: &mut Inv, max_stack: i32, allow_armor: bool, items: Vec<Item>) -> Vec<Item> {
    let items = normalize(items);
    let (mut empty, mut partial) = inventory_data(host, inv, &items, max_stack, allow_armor);
    let mut leftover = Vec::new();
    for (k, mut item) in items.into_iter().enumerate() {
        if item.count <= 0 {
            continue;
        }
        let max = item_max(host, &item, max_stack);
        loop {
            if partial[k].is_empty() {
                if empty.is_empty() {
                    leftover.push(item);
                    break;
                }
                let slot = empty.remove(0);
                if item.count > max {
                    let mut split = item.clone();
                    split.count = max;
                    inv.slots[slot] = Some(split);
                    item.count -= max;
                } else {
                    inv.slots[slot] = Some(item);
                    break;
                }
            } else {
                let slot = partial[k].remove(0);
                let existing = inv.get(slot).map_or(0, |i| i.count);
                let mut stack = inv.get(slot).cloned().unwrap_or_else(|| item.clone());
                if item.count + existing <= max {
                    stack.count = item.count + existing;
                    inv.slots[slot] = Some(stack);
                    break;
                }
                stack.count = max;
                inv.slots[slot] = Some(stack);
                item.count = item.count + existing - max;
            }
        }
    }
    leftover
}

/// Inventories.addItem(player, maxStack, item, slot): into one slot; the rest comes back.
fn add_to_slot(host: &mut dyn Host, inv: &mut Inv, max_stack: i32, mut item: Item, slot: usize) -> Option<Item> {
    if slot >= 41 {
        return Some(item);
    }
    let max = item_max(host, &item, max_stack);
    let (mut existing, have) = match inv.get(slot) {
        Some(e) => (e.clone(), e.count),
        None => (item.clone(), 0),
    };
    if !similar(&item, &existing) {
        return Some(item);
    }
    if item.count + have <= max {
        existing.count = item.count + have;
        inv.slots[slot] = Some(existing);
        return None;
    }
    existing.count = max;
    inv.slots[slot] = Some(existing);
    item.count = item.count + have - max;
    Some(item)
}

/// The oversized stack size a player may get (0: the item's own).
fn oversized(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> i32 {
    if ess.has(host, Sender::Player(player), "essentials.oversizedstacks") { ess.config_i64("oversized-stacksize", 64) as i32 } else { 0 }
}

/// Gives items to a player (Inventories.addItem with their oversized stack size); what doesn't fit is dropped
/// at their feet when `drop`, else returned.
pub fn add_items(ess: &mut Ess, host: &mut dyn Host, player: Uuid, items: Vec<Item>, drop: bool) -> Vec<Item> {
    let max = oversized(ess, host, player);
    let mut inv = Inv::of(host, player);
    let left = add_to(host, &mut inv, max, false, items);
    inv.save(host);
    if drop {
        if let Some(at) = host.location(player) {
            for it in &left {
                host.drop_item(&at, it);
            }
        }
        return Vec::new();
    }
    left
}

fn remove_where(inv: &mut Inv, include_armor: bool, mut pred: impl FnMut(&Item) -> bool) -> i32 {
    let mut removed = 0;
    for i in 0..41 {
        if !include_armor && is_armor_slot(i) {
            continue;
        }
        if let Some(it) = inv.get(i) {
            if pred(it) {
                removed += it.count;
                inv.slots[i] = None;
            }
        }
    }
    removed
}

/// Inventories.removeItemAmount: all or nothing.
fn remove_amount(inv: &mut Inv, item: &Item, mut amount: i32) -> bool {
    let mut clear = Vec::new();
    for i in 0..41 {
        let Some(have) = inv.get(i).cloned() else { continue };
        if !similar(&have, item) {
            continue;
        }
        if have.count >= amount {
            let mut left = have;
            left.count -= amount;
            inv.slots[i] = (left.count > 0).then_some(left);
            for s in clear {
                inv.slots[s] = None;
            }
            return true;
        }
        amount -= have.count;
        clear.push(i);
        if amount == 0 {
            for s in clear {
                inv.slots[s] = None;
            }
            return true;
        }
    }
    false
}

// ===== Kits (Kit, Kits) =====================================================================================

/// Every kit: (lower-case name, section, file it lives in), kits.yml first, then kits/*.yml.
fn all_kits(ess: &Ess) -> Vec<(String, Value, String)> {
    let mut files = vec!["kits.yml".to_string()];
    if let Ok(dir) = std::fs::read_dir(ess.dir.join("kits")) {
        let mut extra: Vec<String> = dir.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".yml")).map(|n| format!("kits/{n}")).collect();
        extra.sort();
        files.extend(extra);
    }
    let mut out: Vec<(String, Value, String)> = Vec::new();
    for f in files {
        if let Some(Value::Object(kits)) = ess.read_yaml(&f).get("kits") {
            for (name, section) in kits {
                if section.is_object() {
                    let lower = name.to_lowercase();
                    out.retain(|(n, _, _)| *n != lower);
                    out.push((lower, section.clone(), f.clone()));
                }
            }
        }
    }
    out
}

fn kit_key(name: &str) -> String {
    name.replace(['.', '/'], "_")
}

/// Kits.getKit: by its (lower-case) key.
fn find_kit(ess: &Ess, name: &str) -> Option<(String, Value, String)> {
    let key = kit_key(name);
    all_kits(ess).into_iter().find(|(n, _, _)| *n == key)
}

fn kit_timestamp(ess: &mut Ess, player: Uuid, kit: &str) -> i64 {
    let key = kit_key(kit).to_lowercase();
    ess.users.get(player).and_then(|u| u.i64(&format!("timestamps.kits.{key}"))).unwrap_or(0)
}
fn set_kit_timestamp(ess: &mut Ess, player: Uuid, kit: &str, time: i64) {
    let key = kit_key(kit).to_lowercase();
    ess.users.user(player).set(&format!("timestamps.kits.{key}"), json!(time));
}

/// Kit.getNextUse: 0 usable, -1 used one-time kit, else the time it can be used again.
fn kit_next_use(ess: &mut Ess, host: &mut dyn Host, player: Uuid, kit: &str, section: &Value) -> Result<i64, CmdError> {
    if ess.has(host, Sender::Player(player), "essentials.kit.exemptdelay") {
        return Ok(0);
    }
    let delay = match section.get("delay") {
        None | Some(Value::Null) => 0.0,
        Some(v) => v.as_f64().ok_or_else(|| tl("kitError2", vec![]))?,
    };
    let now = host.now_millis();
    let last = kit_timestamp(ess, player, kit);
    let next = last + (delay * 1000.0) as i64;
    if last == 0 || last > now {
        Ok(0)
    } else if delay < 0.0 {
        Ok(-1)
    } else if next < now {
        Ok(0)
    } else {
        Ok(next)
    }
}

/// Kit.checkDelay: kitOnce / kitTimed, then a silent stop.
fn kit_check_delay(ess: &mut Ess, host: &mut dyn Host, player: Uuid, kit: &str, section: &Value) -> Result<(), CmdError> {
    let next = kit_next_use(ess, host, player, kit, section)?;
    if next == 0 {
        return Ok(());
    }
    if next < 0 {
        ess.send_tl(host, Sender::Player(player), "kitOnce", &[]);
    } else {
        let now = host.now_millis();
        let when = format_date_diff(ess, now, next);
        ess.send_tl(host, Sender::Player(player), "kitTimed", &[plain(when)]);
    }
    Err(CmdError::Quiet)
}

fn kit_items(section: &Value) -> Result<Vec<String>, CmdError> {
    match section.get("items") {
        Some(Value::Array(a)) => a.iter().map(|v| v.as_str().map(str::to_string).ok_or_else(|| tl("kitError2", vec![]))).collect(),
        _ => Err(tl("kitError2", vec![])),
    }
}

/// Kit.expandItems: the kit's items into the player's inventory (slots, auto-equip, drop or refuse when full),
/// then its money and commands. False when the inventory had no room.
fn expand_kit(ess: &mut Ess, host: &mut dyn Host, player: Uuid, kit: &str, section: &Value) -> Result<bool, CmdError> {
    let lines = kit_items(section)?;
    let name = host.name(player).unwrap_or_default();
    let allow_unsafe = ess.config_bool("unsafe-enchantments", false);
    let auto_equip = ess.config_bool("kit-auto-equip", false);
    let symbol = ess.config_str("currency-symbol").filter(|s| !s.is_empty()).unwrap_or_else(|| "$".into());
    let mut items: Vec<Item> = Vec::new();
    let mut slotted: Vec<(usize, Item)> = Vec::new();
    let mut commands = Vec::new();
    let mut money = Vec::new();
    let fail = |e: CmdError| match e {
        CmdError::Tl("kitError2", _) => e,
        _ => tl("kitError2", vec![]),
    };
    for raw in lines {
        let line = raw.replace("{USERNAME}", &name).replace("{PLAYER}", &name).replace("{DISPLAYNAME}", &name);
        if line.starts_with('$') || line.starts_with(symbol.as_str()) {
            money.push(line.trim_start_matches('$').trim_start_matches(symbol.as_str()).replace(',', ""));
            continue;
        }
        if let Some(cmd) = line.strip_prefix('/') {
            commands.push(cmd.replace("{player}", &name));
            continue;
        }
        let mut rest = line.as_str();
        let mut slot = None;
        if let Some(after) = rest.strip_prefix("slot:") {
            if let Some((n, tail)) = after.split_once(' ') {
                slot = n.parse::<usize>().ok();
                rest = tail;
            }
        }
        if rest.starts_with('@') {
            let msg = ess.tl("kitError3", &[plain(kit), plain(&name)]);
            ess.send_mini(host, Sender::Console, &msg);
            continue;
        }
        let Some(stack) = parse_item_line(ess, host, None, rest, allow_unsafe).map_err(fail)? else { continue };
        match slot {
            Some(s) if !slotted.iter().any(|(x, _)| *x == s) => slotted.push((s, stack)),
            _ => items.push(stack),
        }
    }
    let max = oversized(ess, host, player);
    let drop_full = ess.config_bool("drop-items-if-full", false);
    let mut inv = Inv::of(host, player);
    let all: Vec<Item> = items.iter().cloned().chain(slotted.iter().map(|(_, i)| i.clone())).collect();
    if !drop_full && !has_space(host, &inv, max, auto_equip, &all) {
        ess.send_tl(host, Sender::Player(player), "kitInvFullNoDrop", &[]);
        return Ok(false);
    }
    for (slot, it) in slotted {
        if let Some(left) = add_to_slot(host, &mut inv, max, it, slot) {
            items.push(left);
        }
    }
    let leftover = add_to(host, &mut inv, max, auto_equip, items);
    inv.save(host);
    let mut spew = false;
    if let Some(at) = host.location(player) {
        for it in leftover {
            let mut amount = it.count;
            while amount > 0 {
                let mut drop = it.clone();
                if max != 0 {
                    drop.count = amount.min(host.max_stack(&it.id).max(1));
                }
                host.drop_item(&at, &drop);
                amount -= drop.count;
            }
            spew = true;
        }
    }
    for m in money {
        let line = format!("eco give {name} {}", m.trim());
        if ess.claims(&line) {
            ess.dispatch(host, Sender::Console, &line);
        }
    }
    for cmd in commands {
        run_console(ess, host, &cmd);
    }
    if spew {
        ess.send_tl(host, Sender::Player(player), "kitInvFull", &[]);
    }
    Ok(true)
}

/// Bukkit.dispatchCommand(console, line): Essentials' commands, else vanilla's.
fn run_console(ess: &mut Ess, host: &mut dyn Host, line: &str) {
    if ess.claims(line) {
        ess.dispatch(host, Sender::Console, line);
    } else {
        host.console_command(line);
    }
}

/// player.chat("/" + line): Essentials' commands, else vanilla's with the player's permissions.
fn run_as_player(ess: &mut Ess, host: &mut dyn Host, player: Uuid, line: &str) {
    if ess.claims(line) {
        ess.dispatch(host, Sender::Player(player), line);
    } else {
        host.player_command(player, line, false);
    }
}

/// Gives a kit to a player without permission, delay or cost checks (other areas: [Kit] signs, first-join
/// kits). Ok(false) when the inventory had no room (and drop-items-if-full is off).
pub fn give_kit(ess: &mut Ess, host: &mut dyn Host, player: Uuid, kit: &str) -> Result<bool, CmdError> {
    let (name, section, _) = find_kit(ess, &kit.to_lowercase()).ok_or_else(|| tl("kitNotFound", vec![]))?;
    expand_kit(ess, host, player, &name, &section)
}

/// Every kit name (lower-case).
pub fn kit_names(ess: &Ess) -> Vec<String> {
    all_kits(ess).into_iter().map(|(n, _, _)| n).collect()
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Kits.listKits: the kits a player may use (struck through while on cooldown), or every kit for the console.
fn list_kits(ess: &mut Ess, host: &mut dyn Host, who: Sender) -> Result<String, CmdError> {
    let mut kits = all_kits(ess);
    kits.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::new();
    for (name, section, _) in kits {
        match who {
            Sender::Console => out.push_str(&format!(" {}", capital(&name))),
            Sender::Player(p) => {
                if !ess.has(host, who, &format!("essentials.kits.{name}")) {
                    continue;
                }
                let mut shown = capital(&name);
                let next = kit_next_use(ess, host, p, &name, &section).map_err(|_| tl("kitError", vec![]))?;
                if next == -1 && ess.config_bool("skip-used-one-time-kits-from-kit-list", false) {
                    continue;
                } else if next != 0 {
                    shown = ess.tl("kitDelay", &[plain(shown)]);
                }
                out.push(' ');
                out.push_str(&shown);
            }
        }
    }
    Ok(out.trim().to_string())
}

fn cmd_kit(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let Sender::Player(me) = ctx.sender else {
        if args.len() < 2 {
            let list = list_kits(ctx.ess, ctx.host, Sender::Console)?;
            ctx.tl(if list.is_empty() { "noKits" } else { "kits" }, &[Arg::Parsed(list)]);
            return Err(CmdError::Quiet);
        }
        let to = ctx.online_player(&args[1])?;
        for kit in args[0].to_lowercase().split(',') {
            let (name, section, _) = find_kit(ctx.ess, kit).ok_or_else(|| tl("kitNotFound", vec![]))?;
            expand_kit(ctx.ess, ctx.host, to, &name, &section)?;
            let shown = display_name(ctx, to);
            ctx.tl("kitGiveTo", &[plain(kit), plain(shown)]);
            ctx.tl_to(Sender::Player(to), "kitReceive", &[plain(kit)]);
        }
        return Ok(());
    };
    if args.is_empty() {
        let list = list_kits(ctx.ess, ctx.host, ctx.sender)?;
        ctx.tl(if list.is_empty() { "noKits" } else { "kits" }, &[Arg::Parsed(list)]);
        return Err(CmdError::Quiet);
    }
    let to = if args.len() > 1 && ctx.has("essentials.kit.others") { ctx.online_player(&args[1])? } else { me };
    let names = args[0].to_lowercase().trim().to_string();
    if names.is_empty() {
        return Err(tl("kitNotFound", vec![]));
    }
    let mut kits = Vec::new();
    for kit in names.split(',') {
        if kit.is_empty() {
            return Err(tl("kitNotFound", vec![]));
        }
        let (name, section, _) = find_kit(ctx.ess, kit).ok_or_else(|| tl("kitNotFound", vec![]))?;
        if !ctx.has(&format!("essentials.kits.{name}")) {
            return Err(tl("noKitPermission", vec![plain(format!("essentials.kits.{name}"))]));
        }
        kit_check_delay(ctx.ess, ctx.host, me, &name, &section)?;
        kits.push((name, section));
    }
    for (name, section) in kits {
        if kit_check_delay(ctx.ess, ctx.host, me, &name, &section).is_err() {
            continue;
        }
        match expand_kit(ctx.ess, ctx.host, to, &name, &section) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(e) => {
                let msg = match e {
                    CmdError::Tl(k, a) => ctx.ess.tl(k, &a),
                    CmdError::Msg(m) => m,
                    _ => continue,
                };
                ctx.tl("errorWithMessage", &[Arg::Parsed(msg)]);
                continue;
            }
        }
        let now = ctx.host.now_millis();
        set_kit_timestamp(ctx.ess, me, &name, now);
        if to != me {
            let shown = display_name(ctx, to);
            ctx.tl("kitGiveTo", &[plain(&name), plain(shown)]);
        }
        ctx.tl_to(Sender::Player(to), "kitReceive", &[plain(&name)]);
    }
    Ok(())
}

fn cmd_kitreset(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let min = if ctx.is_player() { 1 } else { 2 };
    if args.len() < min {
        return Err(CmdError::Usage(None));
    }
    let kit = &args[0];
    if find_kit(ctx.ess, kit).is_none() {
        return Err(tl("kitNotFound", vec![]));
    }
    let me = ctx.sender.player();
    let target = if !ctx.is_player() || (args.len() > 1 && ctx.has("essentials.kitreset.others")) { ctx.online_player(&args[1])? } else { me.expect("a player") };
    set_kit_timestamp(ctx.ess, target, kit, 0);
    if Some(target) == me {
        ctx.tl("kitReset", &[plain(kit)]);
    } else {
        let shown = display_name(ctx, target);
        ctx.tl("kitResetOther", &[plain(kit), plain(shown)]);
    }
    Ok(())
}

fn cmd_showkit(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    ctx.player()?;
    if args.len() != 1 {
        return Err(CmdError::Usage(None));
    }
    for kit in args[0].to_lowercase().split(',') {
        ctx.tl("kitContains", &[plain(kit)]);
        let (_, section, _) = find_kit(ctx.ess, kit).ok_or_else(|| tl("kitNotFound", vec![]))?;
        for line in kit_items(&section)? {
            ctx.tl("kitItem", &[plain(line)]);
        }
    }
    Ok(())
}

fn cmd_createkit(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.len() != 2 {
        return Err(CmdError::Usage(None));
    }
    let delay: i64 = args[1].parse().map_err(|_| CmdError::Usage(None))?;
    let inv = Inv::of(ctx.host, me);
    let mut list = Vec::new();
    for (i, slot) in inv.slots.iter().enumerate() {
        if let Some(item) = slot {
            list.push(format!("slot:{i} {}", serialize_item(item)));
        }
    }
    if ctx.ess.config_bool("pastebin-createkit", false) {
        // No paste service from a vanilla server: the kit is written to kits.yml instead (see PARITY).
    }
    let name = kit_key(&args[0]).to_lowercase();
    let mut file = ctx.ess.read_yaml("kits.yml");
    if !file.is_object() {
        file = json!({});
    }
    crate::game::ess_yaml::set(&mut file, &format!("kits.{name}.delay"), json!(delay));
    crate::game::ess_yaml::set(&mut file, &format!("kits.{name}.items"), json!(list));
    ctx.ess.write_yaml("kits.yml", &file);
    ctx.tl("createdKit", &[plain(&args[0]), plain(list.len().to_string()), plain(delay.to_string())]);
    Ok(())
}

fn cmd_delkit(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        let list = list_kits(ctx.ess, ctx.host, Sender::Console)?;
        ctx.tl(if list.is_empty() { "noKits" } else { "kits" }, &[plain(list)]);
        return Err(CmdError::Quiet);
    }
    let found = all_kits(ctx.ess).into_iter().find(|(n, _, _)| n.eq_ignore_ascii_case(&args[0]));
    let Some((name, _, file)) = found else { return Err(tl("kitNotFound", vec![])) };
    if ctx.is_player() && !ctx.has(&format!("essentials.kits.{name}")) {
        return Err(tl("noKitPermission", vec![plain(format!("essentials.kits.{name}"))]));
    }
    let mut yaml = ctx.ess.read_yaml(&file);
    if let Some(Value::Object(kits)) = yaml.get_mut("kits") {
        let key = kits.keys().find(|k| k.to_lowercase() == name).cloned();
        if let Some(k) = key {
            kits.remove(&k);
        }
    }
    ctx.ess.write_yaml(&file, &yaml);
    ctx.tl("deleteKit", &[plain(&name)]);
    Ok(())
}

// ===== Item commands ======================================================================================

/// The amount /item and /give use: the typed one, else default-stack-size, else the oversized size.
fn spawn_amount(ess: &mut Ess, host: &mut dyn Host, who: Sender, item: &mut Item, arg: Option<&String>) -> Result<(), CmdError> {
    if let Some(a) = arg {
        let n: i32 = a.parse().map_err(|_| CmdError::Usage(None))?;
        if n > 0 {
            item.count = n;
            return Ok(());
        }
    }
    let default = ess.config_i64("default-stack-size", -1);
    let over = ess.config_i64("oversized-stacksize", 64);
    if default > 0 {
        item.count = default as i32;
    } else if over > 0 && ess.has(host, who, "essentials.oversizedstacks") {
        item.count = over as i32;
    }
    Ok(())
}

fn allow_unsafe(ctx: &mut Ctx) -> bool {
    ctx.ess.config_bool("unsafe-enchantments", false) && ctx.has("essentials.enchantments.allowunsafe")
}

fn cmd_item(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let mut stack = item_stack(ctx.ess, ctx.host, &args[0])?;
    if !can_spawn_item(ctx.ess, ctx.host, ctx.sender, &stack) {
        return Err(tl("cantSpawnItem", vec![plain(mat_compact(&stack))]));
    }
    spawn_amount(ctx.ess, ctx.host, ctx.sender, &mut stack, args.get(1))?;
    if args.len() > 2 {
        let unsafe_ = allow_unsafe(ctx);
        let mut meta = Meta::new(stack);
        meta.parse_all(ctx.ess, ctx.host, Some(ctx.sender), unsafe_, args, 2)?;
        stack = meta.item;
    }
    if bare(&stack.id) == "air" {
        return Err(tl("cantSpawnItem", vec![plain("Air")]));
    }
    ctx.tl("itemSpawn", &[plain(stack.count.to_string()), plain(mat_spaced(&stack))]);
    add_items(ctx.ess, ctx.host, me, vec![stack], false);
    Ok(())
}

fn cmd_give(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.len() < 2 {
        return Err(CmdError::Usage(None));
    }
    let mut stack = item_stack(ctx.ess, ctx.host, &args[1])?;
    if !can_spawn_item(ctx.ess, ctx.host, ctx.sender, &stack) {
        return Err(tl("cantSpawnItem", vec![plain(mat_compact(&stack))]));
    }
    spawn_amount(ctx.ess, ctx.host, ctx.sender, &mut stack, args.get(2))?;
    if args.len() > 3 {
        let unsafe_ = ctx.ess.config_bool("unsafe-enchantments", false) && (!ctx.is_player() || ctx.has("essentials.enchantments.allowunsafe"));
        let start = if is_int(&args[3]) { 4 } else { 3 };
        if args.len() > start {
            let mut meta = Meta::new(stack);
            meta.parse_all(ctx.ess, ctx.host, Some(ctx.sender), unsafe_, args, start)?;
            stack = meta.item;
        }
    }
    if bare(&stack.id) == "air" {
        return Err(tl("cantSpawnItem", vec![plain("Air")]));
    }
    let name = mat_spaced(&stack);
    let drop = ctx.ess.config_bool("drop-items-if-full", false);
    for p in loop_players(ctx, &args[0])? {
        let shown = display_name(ctx, p);
        ctx.tl("giveSpawn", &[plain(stack.count.to_string()), plain(&name), plain(&shown)]);
        let left = add_items(ctx.ess, ctx.host, p, vec![stack.clone()], drop);
        for it in left {
            ctx.tl("giveSpawnFailure", &[plain(it.count.to_string()), plain(&name), plain(&shown)]);
        }
    }
    Ok(())
}

fn cmd_more(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let (slot, item) = held(ctx.host, me);
    let Some(mut stack) = item else { return Err(tl("cantSpawnItem", vec![plain("Air")])) };
    let over = ctx.has("essentials.oversizedstacks");
    let max = ctx.host.max_stack(&stack.id).max(1);
    let cap = if over { ctx.ess.config_i64("oversized-stacksize", 64) as i32 } else { max };
    if stack.count >= cap {
        return Err(tl("fullStack", vec![]));
    }
    if !can_spawn_item(ctx.ess, ctx.host, ctx.sender, &stack) {
        return Err(tl("cantSpawnItem", vec![plain(mat_compact(&stack))]));
    }
    let size = if let Some(a) = args.first() {
        let n: i64 = a.parse().ok().filter(|n| *n > 0).ok_or_else(|| tl("nonZeroPosNumber", vec![]))?;
        let want = stack.count as i64 + n;
        if want > cap as i64 {
            ctx.tl(if over { "fullStackDefaultOversize" } else { "fullStackDefault" }, &[plain(cap.to_string())]);
            cap
        } else {
            want as i32
        }
    } else {
        cap
    };
    stack.count = size;
    ctx.host.set_slot(me, slot, Some(stack));
    Ok(())
}

fn cmd_itemdb(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let (stack, held_item) = match args.first() {
        Some(name) => (item_stack(ctx.ess, ctx.host, name)?, false),
        None => {
            let me = ctx.sender.player().ok_or(CmdError::Usage(None))?;
            match held(ctx.host, me).1 {
                Some(i) => (i, true),
                None => (Item::new("air", 0), true),
            }
        }
    };
    ctx.tl("itemType", &[plain(mat_upper(&stack)), plain("none")]);
    if held_item && bare(&stack.id) != "air" {
        let max = ctx.host.max_damage(&stack.id);
        if max != 0 {
            let left = (max as i64 + 1) - damage_of(&stack);
            ctx.tl("durability", &[plain(left.to_string())]);
        }
    }
    if bare(&stack.id) == "air" {
        return Ok(());
    }
    let mut names = item_names(&stack);
    let custom = ctx.ess.read_yaml("custom_items.yml");
    let primary = item_db_name(&stack);
    if let Some(Value::Object(aliases)) = custom.get("aliases") {
        for (alias, target) in aliases {
            if target.as_str().is_some_and(|t| t.eq_ignore_ascii_case(&primary)) {
                names.push(alias.clone());
            }
        }
    }
    names.sort();
    if names.len() > 15 {
        names.truncate(14);
    }
    ctx.tl("itemNames", &[plain(names.join(", "))]);
    Ok(())
}

fn cmd_itemname(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let (slot, item) = held(ctx.host, me);
    let Some(mut item) = item else {
        ctx.tl("itemnameInvalidItem", &[]);
        return Ok(());
    };
    let node = format!("essentials.itemname.prevent-type.{}", bare(&item.id));
    let wildcard = exact_perm(ctx, "essentials.itemname.prevent-type.*", "essentials.itemname.__probe__");
    let material = exact_perm(ctx, &node, "essentials.itemname.__probe__");
    if (wildcard == Some(true) && material != Some(false)) || (wildcard != Some(true) && material == Some(true)) {
        ctx.tl("itemnameInvalidItem", &[]);
        return Ok(());
    }
    let name = format_string(ctx, "essentials.itemname", &final_arg(args, 0)).trim().to_string();
    if name.is_empty() {
        set_comp(&mut item, "custom_name", None);
        ctx.host.set_slot(me, slot, Some(item));
        ctx.tl("itemnameClear", &[]);
    } else {
        set_comp(&mut item, "custom_name", Some(legacy_component(&name)));
        ctx.host.set_slot(me, slot, Some(item));
        ctx.tl("itemnameSuccess", &[plain(name)]);
    }
    Ok(())
}

fn cmd_itemlore(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let (slot, item) = held(ctx.host, me);
    let Some(mut item) = item else { return Err(tl("itemloreInvalidItem", vec![])) };
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let mut lore: Vec<Value> = comp(&item, "lore").and_then(Value::as_array).cloned().unwrap_or_default();
    let sub = args[0].to_lowercase();
    if sub == "add" && args.len() > 1 {
        let max = ctx.ess.config_i64("max-itemlore-lines", 10) as usize;
        if lore.len() >= max && !ctx.has("essentials.itemlore.bypass") {
            return Err(tl("itemloreMaxLore", vec![]));
        }
        let line = format_string(ctx, "essentials.itemlore", &final_arg(args, 1)).trim().to_string();
        lore.push(legacy_component(&line));
        set_comp(&mut item, "lore", Some(json!(lore)));
        ctx.host.set_slot(me, slot, Some(item));
        ctx.tl("itemloreSuccess", &[plain(line)]);
    } else if sub == "clear" {
        set_comp(&mut item, "lore", None);
        ctx.host.set_slot(me, slot, Some(item));
        ctx.tl("itemloreClear", &[]);
    } else if sub == "set" && args.len() > 2 && is_int(&args[1]) {
        if lore.is_empty() {
            return Err(tl("itemloreNoLore", vec![]));
        }
        let n: i32 = args[1].parse().unwrap_or(0);
        let line = format_string(ctx, "essentials.itemlore", &final_arg(args, 2)).trim().to_string();
        if n < 1 || n as usize > lore.len() {
            return Err(tl("itemloreNoLine", vec![plain(n.to_string())]));
        }
        lore[n as usize - 1] = legacy_component(&line);
        set_comp(&mut item, "lore", Some(json!(lore)));
        ctx.host.set_slot(me, slot, Some(item));
        ctx.tl("itemloreSuccessLore", &[plain(n.to_string()), plain(line)]);
    } else {
        return Err(CmdError::Usage(None));
    }
    Ok(())
}

fn enchant_list(ctx: &mut Ctx, item: Option<&Item>) -> String {
    let mut usable: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let all = ctx.host.enchantments();
    for (name, path, primary) in db::ENCHANTMENTS.iter() {
        if !*primary {
            continue;
        }
        let id = ns(path);
        if !all.is_empty() && !all.iter().any(|(e, _)| *e == id) {
            continue;
        }
        let ok = seen.contains(&id.to_string()) || (ctx.has(&format!("essentials.enchantments.{path}")) && item.is_none_or(|i| ctx.host.can_enchant(&i.id, &id)));
        if ok {
            seen.push(id);
            usable.push(name.to_string());
        }
    }
    usable.sort();
    usable.dedup();
    usable.join(", ")
}

fn cmd_enchant(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let (slot, item) = held(ctx.host, me);
    let Some(mut item) = item else { return Err(tl("nothingInHand", vec![])) };
    if args.is_empty() {
        let list = enchant_list(ctx, Some(&item));
        let line = ctx.ess.tl("enchantments", &[plain(list)]);
        return Err(CmdError::Usage(Some(line)));
    }
    let level: i32 = match args.get(1) {
        Some(l) => l.parse().map_err(|_| CmdError::Usage(None))?,
        None => 1,
    };
    let Some((id, max)) = enchantment(ctx.host, &args[0]) else { return Err(tl("enchantmentNotFound", vec![])) };
    if !ctx.has(&format!("essentials.enchantments.{}", bare(&id))) {
        return Err(tl("enchantmentPerm", vec![plain(bare(&id))]));
    }
    let unsafe_ = allow_unsafe(ctx);
    add_enchantment(ctx.host, &mut item, &id, max, level, unsafe_)?;
    ctx.host.set_slot(me, slot, Some(item));
    let name = bare(&id).replace('_', " ");
    ctx.tl(if level == 0 { "enchantmentRemoved" } else { "enchantmentApplied" }, &[plain(name)]);
    Ok(())
}

fn cmd_repair(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let all = args.first().map(|a| a.to_lowercase());
    if all.is_none() || all.as_deref() == Some("hand") || !ctx.has("essentials.repair.all") {
        let (slot, item) = held(ctx.host, me);
        let Some(mut item) = item else { return Err(tl("repairInvalidType", vec![])) };
        let max = ctx.host.max_damage(&item.id);
        if max < 1 || damage_of(&item) == 0 {
            return Err(tl("repairInvalidType", vec![]));
        }
        if !enchants_of(&item).is_empty() && !ctx.ess.config_bool("repair-enchanted", true) && !ctx.has("essentials.repair.enchanted") {
            return Err(tl("repairEnchanted", vec![]));
        }
        set_comp(&mut item, "damage", None);
        let name = mat_spaced(&item);
        ctx.host.set_slot(me, slot, Some(item));
        ctx.tl("repair", &[plain(name)]);
        return Ok(());
    }
    if all.as_deref() != Some("all") {
        return Err(CmdError::Usage(None));
    }
    let mut inv = Inv::of(ctx.host, me);
    let mut repaired = Vec::new();
    let armor = ctx.has("essentials.repair.armor");
    let enchanted_ok = ctx.ess.config_bool("repair-enchanted", true) || ctx.has("essentials.repair.enchanted");
    let mut order: Vec<usize> = (0..36).chain([OFFHAND]).collect();
    if armor {
        order.extend([BOOTS, LEGS, CHEST, HELM]);
    }
    for i in order {
        let Some(item) = inv.get(i).cloned() else { continue };
        if ctx.host.max_damage(&item.id) < 1 || damage_of(&item) == 0 {
            continue;
        }
        if !enchants_of(&item).is_empty() && !enchanted_ok {
            continue;
        }
        let mut fixed = item;
        set_comp(&mut fixed, "damage", None);
        repaired.push(mat_spaced(&fixed));
        inv.slots[i] = Some(fixed);
    }
    inv.save(ctx.host);
    if repaired.is_empty() {
        return Err(tl("repairNone", vec![]));
    }
    ctx.tl("repair", &[plain(repaired.join(", "))]);
    Ok(())
}

fn cmd_hat(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let remove = args.first().is_some_and(|a| a.contains("rem") || a.contains("off") || a == "0");
    let mut inv = Inv::of(ctx.host, me);
    let head = inv.get(HELM).cloned();
    let cursed = |h: &Option<Item>| h.as_ref().is_some_and(|h| enchants_of(h).contains_key("minecraft:binding_curse"));
    if !remove {
        let (slot, hand) = held(ctx.host, me);
        let Some(hand) = hand else {
            ctx.tl("hatFail", &[]);
            return Ok(());
        };
        let wildcard = exact_perm(ctx, "essentials.hat.prevent-type.*", "essentials.hat.__probe__");
        let material = exact_perm(ctx, &format!("essentials.hat.prevent-type.{}", bare(&hand.id)), "essentials.hat.__probe__");
        if (wildcard == Some(true) && material != Some(false)) || (wildcard != Some(true) && material == Some(true)) {
            ctx.tl("hatFail", &[]);
            return Ok(());
        }
        if ctx.host.max_damage(&hand.id) != 0 {
            ctx.tl("hatArmor", &[]);
            return Ok(());
        }
        if cursed(&head) && !ctx.has("essentials.hat.ignore-binding") {
            ctx.tl("hatCurse", &[]);
            return Ok(());
        }
        inv.slots[HELM] = Some(hand);
        inv.slots[slot] = head;
        inv.save(ctx.host);
        ctx.tl("hatPlaced", &[]);
        return Ok(());
    }
    match head {
        None => ctx.tl("hatEmpty", &[]),
        Some(_) if cursed(&head) && !ctx.has("essentials.hat.ignore-binding") => ctx.tl("hatCurse", &[]),
        Some(h) => {
            inv.slots[HELM] = None;
            inv.save(ctx.host);
            add_items(ctx.ess, ctx.host, me, vec![h], false);
            ctx.tl("hatRemoved", &[]);
        }
    }
    Ok(())
}

fn cmd_clearinventoryconfirmtoggle(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let mut on = !prompting_clear(ctx.ess, me);
    if ctx.label.ends_with("on") {
        on = true;
    } else if ctx.label.ends_with("off") {
        on = false;
    }
    ctx.ess.users.user(me).set("confirm-clear", json!(on));
    ctx.tl(if on { "clearInventoryConfirmToggleOn" } else { "clearInventoryConfirmToggleOff" }, &[]);
    ctx.ess.state::<ItemsState>().confirming_clear.remove(&me);
    Ok(())
}

fn prompting_clear(ess: &mut Ess, player: Uuid) -> bool {
    match ess.users.get(player).and_then(|u| u.get("confirm-clear").and_then(Value::as_bool)) {
        Some(b) => b,
        None => ess.config_list("default-enabled-confirm-commands").iter().any(|c| c.eq_ignore_ascii_case("clearinventory")),
    }
}

fn cmd_clearinventory(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let (others, all) = match ctx.sender {
        Sender::Console => (true, true),
        Sender::Player(_) => (ctx.has("essentials.clearinventory.others"), ctx.has("essentials.clearinventory.all") || ctx.has("essentials.clearinventory.multiple")),
    };
    let me = ctx.sender.player();
    let mut players: Vec<Uuid> = me.into_iter().collect();
    let mut previous = String::new();
    if let Some(p) = me {
        previous = ctx.ess.state::<ItemsState>().confirming_clear.remove(&p).unwrap_or_default();
    }
    let mut offset = 0;
    if all && args.first().is_some_and(|a| a == "*") {
        ctx.tl("inventoryClearingFromAll", &[]);
        offset = 1;
        players = ctx.host.online();
    } else if others && args.first().is_some_and(|a| a.trim().chars().count() > 2) {
        offset = 1;
        players = match_players(ctx, args[0].trim());
    }
    if players.is_empty() {
        return Err(CmdError::PlayerNotFound);
    }
    let formatted = if args.is_empty() { format!("/{}", ctx.label) } else { format!("/{} {}", ctx.label, args.join(" ")) };
    if let Some(p) = me {
        if prompting_clear(ctx.ess, p) && formatted != previous {
            ctx.ess.state::<ItemsState>().confirming_clear.insert(p, formatted.clone());
            ctx.tl("confirmClear", &[plain(formatted)]);
            return Ok(());
        }
    }
    let extended = players.len() < 8;
    for p in players {
        clear_handler(ctx, p, args, offset, extended)?;
    }
    Ok(())
}

fn clear_handler(ctx: &mut Ctx, player: Uuid, args: &[String], offset: usize, extended: bool) -> CmdResult {
    let amount: i32 = args.get(offset + 1).and_then(|a| a.parse().ok()).unwrap_or(-1);
    let shown = display_name(ctx, player);
    let spec = args.get(offset).map(|s| s.as_str());
    match spec {
        None | Some("*") | Some("**") => {
            let armor = spec == Some("**");
            if extended {
                ctx.tl(if armor { "inventoryClearingAllArmor" } else { "inventoryClearingAllItems" }, &[plain(&shown)]);
            }
            let mut inv = Inv::of(ctx.host, player);
            remove_where(&mut inv, armor, |_| true);
            inv.save(ctx.host);
        }
        Some(list) => {
            let mut items = Vec::new();
            for part in list.split(',') {
                let name = part.split(':').next().unwrap_or("");
                if let Ok(it) = item_stack(ctx.ess, ctx.host, name) {
                    items.push(Item::new(&it.id, 1));
                }
            }
            for it in items {
                if amount < -1 {
                    return Err(tl("cannotRemoveNegativeItems", vec![]));
                }
                let name = bare(&it.id).to_string();
                let mut inv = Inv::of(ctx.host, player);
                if amount == -1 {
                    let removed = remove_where(&mut inv, true, |x| similar(x, &it));
                    inv.save(ctx.host);
                    if removed > 0 || extended {
                        ctx.tl("inventoryClearingStack", &[plain(removed.to_string()), plain(&name), plain(&shown)]);
                    }
                } else if remove_amount(&mut inv, &it, amount) {
                    inv.save(ctx.host);
                    ctx.tl("inventoryClearingStack", &[plain(amount.to_string()), plain(&name), plain(&shown)]);
                } else if extended {
                    ctx.tl("inventoryClearFail", &[plain(&shown), plain(amount.to_string()), plain(&name)]);
                }
            }
        }
    }
    Ok(())
}

// ----- unlimited ----------------------------------------------------------------------------------------

fn unlimited_of(ess: &mut Ess, player: Uuid) -> Vec<String> {
    ess.users.get(player).and_then(|u| u.get("unlimited").and_then(Value::as_array).cloned()).unwrap_or_default().iter().filter_map(|v| v.as_str().map(str::to_string)).collect()
}

/// User.hasUnlimited(item).
pub fn has_unlimited(ess: &mut Ess, player: Uuid, item: &Item) -> bool {
    let m = mat_upper(item);
    unlimited_of(ess, player).iter().any(|u| *u == m)
}

fn cmd_unlimited(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let target = if args.len() > 1 && ctx.has("essentials.unlimited.others") { ctx.online_player(&args[1])? } else { me };
    match args[0].to_lowercase().as_str() {
        "list" => {
            let items = unlimited_of(ctx.ess, target);
            let mut out = format!("{} ", ctx.ess.tl("unlimitedItems", &[]));
            if items.is_empty() {
                out.push_str(&ctx.ess.tl("none", &[]));
            }
            out.push_str(&items.iter().map(|m| m.to_lowercase().replace('_', "")).collect::<Vec<_>>().join(", "));
            ctx.send_mini(&out);
        }
        "clear" => {
            for m in unlimited_of(ctx.ess, target) {
                toggle_unlimited(ctx, me, target, &m)?;
            }
        }
        _ => toggle_unlimited(ctx, me, target, &args[0])?,
    }
    Ok(())
}

fn toggle_unlimited(ctx: &mut Ctx, me: Uuid, target: Uuid, name: &str) -> CmdResult {
    let mut stack = item_stack(ctx.ess, ctx.host, name)?;
    stack.count = ctx.host.max_stack(&stack.id).min(2);
    stack.components = json!({});
    let compact = mat_compact(&stack);
    let bucket = matches!(bare(&stack.id), "water_bucket" | "lava_bucket");
    if ctx.ess.config_bool("permission-based-item-spawn", false)
        && !ctx.has("essentials.unlimited.item-all")
        && !ctx.has(&format!("essentials.unlimited.item-{compact}"))
        && !(bucket && ctx.has("essentials.unlimited.item-bucket"))
    {
        return Err(tl("unlimitedItemPermission", vec![plain(compact)]));
    }
    let mut list = unlimited_of(ctx.ess, target);
    let m = mat_upper(&stack);
    let enable = !list.contains(&m);
    let key = if enable { "enableUnlimited" } else { "disableUnlimited" };
    if enable {
        let have: i32 = Inv::of(ctx.host, target).slots.iter().flatten().filter(|i| similar(i, &stack)).map(|i| i.count).sum();
        if have < stack.count {
            add_items(ctx.ess, ctx.host, target, vec![stack.clone()], false);
        }
        list.push(m);
    } else {
        list.retain(|x| *x != m);
    }
    let shown = display_name(ctx, target);
    if me != target {
        ctx.tl(key, &[plain(&compact), plain(&shown)]);
    }
    ctx.tl_to(Sender::Player(target), key, &[plain(&compact), plain(&shown)]);
    ctx.ess.users.user(target).set("unlimited", json!(list));
    Ok(())
}

/// A player used their held item (on a block when `on_block`): an unlimited item spent by placing a block
/// (survival), throwing an egg or emptying a bucket comes back next tick (EssentialsBlockListener,
/// onPlayerEggThrow, onPlayerBucketEmpty).
pub fn on_use_item(ess: &mut Ess, host: &mut dyn Host, player: Uuid, on_block: bool) {
    let (slot, Some(item)) = held(host, player) else { return };
    let survival = host.game_mode(player) == GameMode::Survival;
    unlimited_use(ess, player, slot, item, survival, on_block);
}

/// `on_use_item` for a hook that knows the used slot (Bukkit numbering), its item and the game mode itself
/// (packet hooks run while the hub is lent out: only Ess is at hand). The check runs next tick.
pub fn unlimited_use(ess: &mut Ess, player: Uuid, slot: usize, item: Item, survival: bool, on_block: bool) {
    if !has_unlimited(ess, player, &item) {
        return;
    }
    let path = bare(&item.id).to_string();
    let bucket = path.ends_with("_bucket") && path != "milk_bucket";
    let egg = path == "egg";
    if !(bucket || egg || (on_block && survival)) {
        return;
    }
    ess.scheduler.later(
        1,
        Box::new(move |_ess, host| {
            let now = host.inventory(player).get(slot).cloned().flatten();
            let spent = match &now {
                None => true,
                Some(n) if similar(n, &item) => n.count < item.count,
                Some(n) => bucket && bare(&n.id) == "bucket",
            };
            if spent {
                host.set_slot(player, slot, Some(item.clone()));
            }
        }),
    );
}

// ----- powertools ---------------------------------------------------------------------------------------

fn powertools_of(ess: &mut Ess, player: Uuid, item: &Item) -> Vec<String> {
    let key = bare(&item.id).to_lowercase();
    ess.users.get(player).and_then(|u| u.get(&format!("powertools.{key}")).and_then(Value::as_array).cloned()).unwrap_or_default().iter().filter_map(|v| v.as_str().map(str::to_string)).collect()
}
fn has_powertools(ess: &mut Ess, player: Uuid) -> bool {
    ess.users.get(player).and_then(|u| u.get("powertools").and_then(Value::as_object).map(|m| !m.is_empty())).unwrap_or(false)
}
fn powertools_enabled(ess: &mut Ess, player: Uuid) -> bool {
    ess.users.get(player).and_then(|u| u.get("powertoolsenabled").and_then(Value::as_bool)).unwrap_or(true)
}
fn set_powertools(ess: &mut Ess, player: Uuid, item: &Item, list: Vec<String>) {
    let key = bare(&item.id).to_lowercase();
    let user = ess.users.user(player);
    let mut all = user.get("powertools").and_then(Value::as_object).cloned().unwrap_or_default();
    if list.is_empty() {
        all.remove(&key);
    } else {
        all.insert(key, json!(list));
    }
    user.set("powertools", Value::Object(all));
}

fn cmd_powertool(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let (user, item, command) = match ctx.sender {
        Sender::Player(p) => (p, held(ctx.host, p).1, final_arg(args, 0)),
        Sender::Console => {
            if args.len() < 3 {
                return Err(CmdError::Msg(format!("When running from console, usage is: /{} <player> <itemid> <command>", ctx.label)));
            }
            let p = ctx.online_player(&args[0])?;
            (p, Some(item_stack(ctx.ess, ctx.host, &args[1])?), final_arg(args, 2))
        }
    };
    if command.eq_ignore_ascii_case("d:") {
        ctx.ess.users.user(user).set("powertools", json!({}));
        ctx.tl("powerToolClearAll", &[]);
        return Ok(());
    }
    let Some(item) = item.filter(|i| bare(&i.id) != "air") else { return Err(tl("powerToolAir", vec![])) };
    let name = mat_spaced(&item);
    let mut tools = powertools_of(ctx.ess, user, &item);
    if !command.is_empty() {
        if command.eq_ignore_ascii_case("l:") {
            if tools.is_empty() {
                return Err(tl("powerToolListEmpty", vec![plain(&name)]));
            }
            ctx.tl("powerToolList", &[plain(tools.join(", ")), plain(&name)]);
            return Err(CmdError::Quiet);
        }
        if let Some(cmd) = command.strip_prefix("r:") {
            if !tools.iter().any(|t| t == cmd) {
                return Err(tl("powerToolNoSuchCommandAssigned", vec![plain(cmd), plain(&name)]));
            }
            tools.retain(|t| t != cmd);
            ctx.tl("powerToolRemove", &[plain(cmd), plain(&name)]);
        } else {
            let cmd = if let Some(c) = command.strip_prefix("a:") {
                if ctx.is_player() && !ctx.has("essentials.powertool.append") {
                    return Err(tl("noPerm", vec![plain("essentials.powertool.append")]));
                }
                if tools.iter().any(|t| t == c) {
                    return Err(tl("powerToolAlreadySet", vec![plain(c), plain(&name)]));
                }
                c.to_string()
            } else {
                tools.clear();
                command.clone()
            };
            tools.push(cmd);
            ctx.tl("powerToolAttach", &[plain(tools.join(", ")), plain(&name)]);
        }
    } else {
        tools.clear();
        ctx.tl("powerToolRemoveAll", &[plain(&name)]);
    }
    if !powertools_enabled(ctx.ess, user) {
        ctx.ess.users.user(user).set("powertoolsenabled", json!(true));
        ctx.tl_to(Sender::Player(user), "powerToolsEnabled", &[]);
    }
    set_powertools(ctx.ess, user, &item, tools);
    Ok(())
}

fn cmd_powertoollist(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if !has_powertools(ctx.ess, me) {
        ctx.tl("noPowerTools", &[]);
        return Ok(());
    }
    let all = ctx.ess.users.user(me).get("powertools").and_then(Value::as_object).cloned().unwrap_or_default();
    for (key, cmds) in all {
        let list: Vec<String> = cmds.as_array().cloned().unwrap_or_default().iter().filter_map(|c| c.as_str().map(str::to_string)).collect();
        ctx.tl("powerToolList", &[plain(list.join(", ")), plain(key.to_lowercase().replace('_', " "))]);
    }
    Ok(())
}

fn cmd_powertooltoggle(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if !has_powertools(ctx.ess, me) {
        ctx.tl("noPowerTools", &[]);
        return Ok(());
    }
    let on = !powertools_enabled(ctx.ess, me);
    ctx.ess.users.user(me).set("powertoolsenabled", json!(on));
    ctx.tl(if on { "powerToolsEnabled" } else { "powerToolsDisabled" }, &[]);
    Ok(())
}

/// A left click in the air or on a block with the held item (EssentialsPlayerListener.usePowertools): its
/// powertool commands run as the player next tick (`c:` lines are chat). True when one ran (the click is
/// then cancelled).
pub fn on_left_click(ess: &mut Ess, host: &mut dyn Host, player: Uuid) -> bool {
    let item = held(host, player).1;
    powertool_click(ess, player, item.as_ref())
}

/// `on_left_click` with the held item given (packet hooks).
pub fn powertool_click(ess: &mut Ess, player: Uuid, held: Option<&Item>) -> bool {
    let Some(item) = held.filter(|i| bare(&i.id) != "air").cloned() else { return false };
    if !has_powertools(ess, player) || !powertools_enabled(ess, player) {
        return false;
    }
    let mut used = false;
    for cmd in powertools_of(ess, player, &item) {
        if cmd.contains("{player}") {
            continue;
        }
        used = true;
        if let Some(chat) = cmd.strip_prefix("c:") {
            let chat = chat.to_string();
            ess.scheduler.later(1, Box::new(move |ess, host| chat_as_player(ess, host, player, &chat)));
        } else {
            ess.scheduler.later(1, Box::new(move |ess, host| run_as_player(ess, host, player, &cmd)));
        }
    }
    used
}

/// player.chat(text): Essentials' chat listeners decide, else it is shown as the player's chat.
fn chat_as_player(ess: &mut Ess, host: &mut dyn Host, player: Uuid, text: &str) {
    match ess.on_chat(host, player, text) {
        Chat::Pass => {
            let name = host.name(player).unwrap_or_default();
            host.chat_as(player, &[Span { text: format!("<{name}> {text}"), style: Default::default() }]);
        }
        Chat::Cancel => {}
        Chat::Replace(lines) => {
            for (to, spans) in lines {
                match to {
                    None => host.broadcast(&spans),
                    Some(list) => {
                        for p in list {
                            host.send(Sender::Player(p), &spans);
                        }
                    }
                }
            }
        }
    }
}

/// A player hit another player (EssentialsEntityListener.onPlayerVsPlayerPowertool): the first powertool
/// command with `{player}` runs with the defender's name. True cancels the hit.
pub fn on_attack_player(ess: &mut Ess, host: &mut dyn Host, attacker: Uuid, defender: Uuid) -> bool {
    let item = held(host, attacker).1;
    let name = host.name(defender).unwrap_or_default();
    powertool_attack(ess, attacker, item.as_ref(), &name)
}

/// `on_attack_player` with the held item and the defender's name given (packet hooks).
pub fn powertool_attack(ess: &mut Ess, attacker: Uuid, held: Option<&Item>, defender: &str) -> bool {
    if !powertools_enabled(ess, attacker) {
        return false;
    }
    let Some(item) = held else { return false };
    let name = defender.to_string();
    for tool in powertools_of(ess, attacker, item) {
        let cmd = tool.replace("{player}", &name);
        if !cmd.is_empty() && cmd != tool {
            ess.scheduler.later(1, Box::new(move |ess, host| run_as_player(ess, host, attacker, &cmd)));
            return true;
        }
    }
    false
}

// ----- book, skull, firework, potion ------------------------------------------------------------------------

fn cmd_book(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let player = ctx.name_of(me);
    let (slot, item) = held(ctx.host, me);
    let Some(mut item) = item else { return Err(tl("holdBook", vec![])) };
    match bare(&item.id) {
        "written_book" => {
            let mut content = comp(&item, "written_book_content").cloned().unwrap_or_else(|| json!({"title": {"raw": ""}, "author": ""}));
            let is_author = content.get("author").and_then(Value::as_str).is_some_and(|a| a.eq_ignore_ascii_case(&player));
            let sub = args.first().map(|a| a.to_lowercase());
            if args.len() > 1 && sub.as_deref() == Some("author") {
                if ctx.has("essentials.book.author") && (is_author || ctx.has("essentials.book.others")) {
                    let author = format_string(ctx, "essentials.book.author", &final_arg(args, 1)).trim().to_string();
                    content["author"] = json!(author);
                    set_comp(&mut item, "written_book_content", Some(content));
                    ctx.host.set_slot(me, slot, Some(item));
                    ctx.tl("bookAuthorSet", &[plain(author)]);
                } else {
                    return Err(tl("denyChangeAuthor", vec![]));
                }
            } else if args.len() > 1 && sub.as_deref() == Some("title") {
                if ctx.has("essentials.book.title") && (is_author || ctx.has("essentials.book.others")) {
                    let title = format_string(ctx, "essentials.book.title", &final_arg(args, 1)).trim().to_string();
                    content["title"] = json!({"raw": title});
                    set_comp(&mut item, "written_book_content", Some(content));
                    ctx.host.set_slot(me, slot, Some(item));
                    ctx.tl("bookTitleSet", &[plain(title)]);
                } else {
                    return Err(tl("denyChangeTitle", vec![]));
                }
            } else if is_author || ctx.has("essentials.book.others") {
                let pages: Vec<Value> = content.get("pages").and_then(Value::as_array).cloned().unwrap_or_default().iter().map(|p| json!({"raw": component_plain(p)})).collect();
                let mut book = Item::new("writable_book", item.count);
                if !pages.is_empty() {
                    set_comp(&mut book, "writable_book_content", Some(json!({"pages": pages})));
                }
                ctx.host.set_slot(me, slot, Some(book));
                ctx.tl("editBookContents", &[]);
            } else {
                return Err(tl("denyBookEdit", vec![]));
            }
        }
        "writable_book" => {
            let pages: Vec<Value> = comp(&item, "writable_book_content").and_then(|c| c.get("pages")).and_then(Value::as_array).cloned().unwrap_or_default();
            let mut book = Item::new("written_book", item.count);
            set_comp(&mut book, "written_book_content", Some(json!({"title": {"raw": ""}, "author": player, "pages": pages})));
            ctx.host.set_slot(me, slot, Some(book));
            ctx.tl("bookLocked", &[]);
        }
        _ => return Err(tl("holdBook", vec![])),
    }
    Ok(())
}

fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn unbase64(s: &str) -> Option<Vec<u8>> {
    const T: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.chars().filter(|c| *c != '=') {
        buf = (buf << 6) | T.find(c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8 & 0xFF);
        }
    }
    Some(out)
}

fn cmd_skull(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let receiver = if args.len() == 2 { ctx.online_player(&args[1])? } else { me };
    let owner: String = if !args.is_empty() && ctx.ess.has(ctx.host, Sender::Player(receiver), "essentials.skull.others") {
        let a = &args[0];
        if a.len() == 180 && a.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)) {
            let decoded = unbase64(a).and_then(|b| String::from_utf8(b).ok()).ok_or_else(|| tl("skullInvalidBase64", vec![]))?;
            let v: Value = serde_json::from_str(&decoded).map_err(|_| tl("skullInvalidBase64", vec![]))?;
            let url = v.pointer("/textures/SKIN/url").and_then(Value::as_str).ok_or_else(|| tl("skullInvalidBase64", vec![]))?;
            let hash = url.rsplit('/').next().unwrap_or("").to_string();
            if !(hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())) {
                return Err(tl("skullInvalidBase64", vec![]));
            }
            hash
        } else if !valid_name(a) {
            return Err(tl("alphaNames", vec![]));
        } else {
            a.clone()
        }
    } else {
        ctx.name_of(me)
    };
    let (slot, hand) = held(ctx.host, me);
    let (mut skull, spawn) = match hand {
        Some(h) if bare(&h.id) == "player_head" && receiver == me => (h, false),
        _ => {
            let node = if receiver == me { "essentials.skull.spawn" } else { "essentials.skull.spawn.others" };
            if !ctx.has(node) {
                return Err(tl("invalidSkull", vec![]));
            }
            (Item::new("player_head", 1), true)
        }
    };
    if comp(&skull, "profile").is_some() && !ctx.has("essentials.skull.modify") {
        return Err(tl("noPermissionSkull", vec![]));
    }
    let texture = owner.len() == 64 && owner.chars().all(|c| c.is_ascii_hexdigit());
    let short = if texture {
        let value = base64(json!({"textures": {"SKIN": {"url": format!("https://textures.minecraft.net/texture/{owner}")}}}).to_string().as_bytes());
        set_comp(&mut skull, "profile", Some(json!({"properties": [{"name": "textures", "value": value}]})));
        owner[..7].to_string()
    } else {
        set_comp(&mut skull, "profile", Some(json!({"name": owner})));
        owner.clone()
    };
    set_comp(&mut skull, "custom_name", Some(legacy_component(&format!("§fSkull of {short}"))));
    if spawn {
        add_items(ctx.ess, ctx.host, receiver, vec![skull], false);
        ctx.tl_to(Sender::Player(receiver), "givenSkull", &[plain(&short)]);
        if receiver != me {
            let shown = display_name(ctx, receiver);
            ctx.tl("givenSkullOther", &[plain(shown), plain(&short)]);
        }
        return Ok(());
    }
    ctx.host.set_slot(me, slot, Some(skull));
    ctx.tl("skullChanged", &[plain(short)]);
    Ok(())
}

fn cmd_firework(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let (slot, stack) = held(ctx.host, me);
    let Some(mut stack) = stack.filter(is_firework) else { return Err(tl("holdFirework", vec![])) };
    let first = args[0].to_lowercase();
    if first == "clear" {
        let mut f = fireworks_of(&stack);
        f["explosions"] = json!([]);
        set_comp(&mut stack, "fireworks", Some(f));
        ctx.host.set_slot(me, slot, Some(stack));
        ctx.tl("fireworkEffectsCleared", &[]);
    } else if args.len() > 1 && (first == "power" || first == "p") {
        let power: i32 = args[1].parse().map_err(|_| tl("invalidFireworkFormat", vec![plain(&args[1]), plain(&args[0])]))?;
        let mut f = fireworks_of(&stack);
        f["flight_duration"] = json!(if power > 3 { 4 } else { power });
        set_comp(&mut stack, "fireworks", Some(f));
        ctx.host.set_slot(me, slot, Some(stack));
    } else if (first == "fire" || first == "f") && ctx.has("essentials.firework.fire") {
        let mut amount = 1;
        let mut direction = false;
        if let Some(a) = args.get(1) {
            if let Ok(n) = a.parse::<i64>() {
                let limit = ctx.ess.config_i64("spawnmob-limit", 10);
                amount = n;
                if amount > limit {
                    amount = limit;
                    ctx.tl("mobSpawnLimit", &[]);
                }
            } else {
                direction = true;
            }
        }
        let at = ctx.host.location(me).ok_or(CmdError::Quiet)?;
        for _ in 0..amount.max(0) {
            let mut item = Item::new(&stack.id, 1);
            item.components = stack.components.clone();
            let mut f = fireworks_of(&item);
            let mut data = Map::new();
            if direction {
                if f["flight_duration"].as_i64().unwrap_or(1) > 1 {
                    f["flight_duration"] = json!(1);
                }
                let (yaw, pitch) = (f64::from(at.yaw).to_radians(), f64::from(at.pitch).to_radians());
                let dir = [-yaw.sin() * pitch.cos() * 0.07, -pitch.sin() * 0.07, yaw.cos() * pitch.cos() * 0.07];
                data.insert("Motion".into(), json!(dir));
                data.insert("ShotAtAngle".into(), json!(true));
            }
            let comps_ns: Map<String, Value> = [("minecraft:fireworks".to_string(), f)].into_iter().collect();
            data.insert("FireworksItem".into(), json!({"id": item.id, "count": 1, "components": comps_ns}));
            ctx.host.spawn_entity("minecraft:firework_rocket", &at, Some(&json_to_snbt(&Value::Object(data))));
        }
    } else {
        let mut meta = Meta::new(stack.clone());
        for arg in args {
            if let Err(e) = meta.firework_meta(ctx.ess, ctx.host, Some(ctx.sender), true, arg) {
                ctx.tl("fireworkSyntax", &[]);
                return Err(e);
            }
        }
        if meta.valid_firework {
            let effect = meta.firework.build()?;
            let mut f = fireworks_of(&meta.item);
            if !f["explosions"].as_array().is_none_or(Vec::is_empty) && !ctx.has("essentials.firework.multiple") {
                return Err(tl("multipleCharges", vec![]));
            }
            f["explosions"].as_array_mut().expect("a list").push(effect);
            set_comp(&mut stack, "fireworks", Some(f));
            ctx.host.set_slot(me, slot, Some(stack));
        } else {
            ctx.tl("fireworkSyntax", &[]);
            return Err(tl("fireworkColor", vec![]));
        }
    }
    Ok(())
}

fn cmd_potion(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let (slot, stack) = held(ctx.host, me);
    if args.is_empty() {
        let mut names: Vec<String> = Vec::new();
        for (name, path, primary) in db::POTION_EFFECTS.iter() {
            if *primary && ctx.has(&format!("essentials.potions.{path}")) {
                names.push(name.to_string());
            }
        }
        names.sort();
        names.dedup();
        let line = ctx.ess.tl("potions", &[plain(names.join(", "))]);
        return Err(CmdError::Usage(Some(line)));
    }
    let Some(mut stack) = stack.filter(is_potion) else { return Err(tl("holdPotion", vec![])) };
    let first = args[0].to_lowercase();
    if first == "clear" {
        if let Some(mut pc) = comp(&stack, "potion_contents").cloned() {
            if let Some(o) = pc.as_object_mut() {
                o.remove("custom_effects");
            }
            let empty = pc.as_object().is_none_or(Map::is_empty);
            set_comp(&mut stack, "potion_contents", (!empty).then_some(pc));
        }
        ctx.host.set_slot(me, slot, Some(stack));
    } else if first == "apply" && ctx.has("essentials.potion.apply") {
        let name = ctx.name_of(me);
        let effects = comp(&stack, "potion_contents").and_then(|p| p.get("custom_effects")).and_then(Value::as_array).cloned().unwrap_or_default();
        for e in effects {
            let id = e.get("id").and_then(Value::as_str).unwrap_or("");
            let duration = e.get("duration").and_then(Value::as_i64).unwrap_or(0);
            let secs = if duration < 0 { "infinite".to_string() } else { (duration / 20).to_string() };
            let amp = e.get("amplifier").and_then(Value::as_i64).unwrap_or(0);
            ctx.host.console_command(&format!("effect give {name} {id} {secs} {amp}"));
        }
    } else if args.len() < 3 {
        return Err(CmdError::Usage(None));
    } else {
        let mut meta = Meta::new(stack);
        for arg in args {
            meta.potion_meta(ctx.ess, ctx.host, Some(ctx.sender), true, arg)?;
        }
        if meta.complete_potion {
            ctx.host.set_slot(me, slot, Some(meta.item));
        } else {
            ctx.tl("invalidPotion", &[]);
            return Err(CmdError::Usage(None));
        }
    }
    Ok(())
}

// ----- exp ------------------------------------------------------------------------------------------------

/// SetExpFix.getExpAtLevel: the points of one level.
fn exp_at_level(level: i64) -> i64 {
    if level <= 15 {
        2 * level + 7
    } else if level <= 30 {
        5 * level - 38
    } else {
        9 * level - 158
    }
}
/// SetExpFix.getExpToLevel: the total points at the start of a level.
fn exp_to_level(level: i64) -> i64 {
    (0..level.max(0)).map(exp_at_level).sum()
}
/// (level, points into it) for a total.
fn level_of(total: i64) -> (i64, i64) {
    let mut level = 0;
    let mut left = total.max(0);
    while left >= exp_at_level(level) {
        left -= exp_at_level(level);
        level += 1;
    }
    (level, left)
}

fn cmd_exp(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.sender.player();
    if args.is_empty() || (args.len() < 2 && me.is_none()) {
        let Some(p) = me else { return Err(CmdError::Usage(None)) };
        show_exp(ctx, p);
        return Ok(());
    }
    let sub = args[0].to_lowercase();
    if !["set", "give", "take", "reset", "show"].contains(&sub.as_str()) {
        return Err(CmdError::Usage(None));
    }
    let perm = |ctx: &mut Ctx, s: &str| !ctx.is_player() || s == "show" || ctx.has(&format!("essentials.exp.{s}"));
    let other = |ctx: &mut Ctx, s: &str| !ctx.is_player() || ctx.has(&format!("essentials.exp.{s}.others"));
    if !perm(ctx, &sub) {
        ctx.tl("noAccessSubCommand", &[plain(format!("/{} {sub}", ctx.label))]);
        return Ok(());
    }
    match sub.as_str() {
        "set" | "give" | "take" => {
            let give = sub != "set";
            let neg = |a: &str| if sub == "take" { format!("-{a}") } else { a.to_string() };
            if args.len() == 3 && other(ctx, &sub) {
                for p in loop_players(ctx, &args[1])? {
                    set_exp(ctx, p, &neg(&args[2]), give)?;
                }
            } else if args.len() == 2 && me.is_some() {
                set_exp(ctx, me.expect("a player"), &neg(&args[1]), give)?;
            } else {
                return Err(CmdError::Usage(None));
            }
        }
        "reset" => {
            if args.len() == 2 && other(ctx, &sub) {
                for p in loop_players(ctx, &args[1])? {
                    set_exp(ctx, p, "0", false)?;
                }
            } else if let Some(p) = me {
                set_exp(ctx, p, "0", false)?;
            } else {
                return Err(CmdError::Usage(None));
            }
        }
        _ => {
            if args.len() == 2 && (!ctx.is_player() || ctx.has("essentials.exp.others")) {
                let p = ctx.online_player(&args[1])?;
                show_exp(ctx, p);
            } else if let Some(p) = me {
                show_exp(ctx, p);
            } else {
                return Err(CmdError::Usage(None));
            }
        }
    }
    Ok(())
}

fn show_exp(ctx: &mut Ctx, p: Uuid) {
    let total = ctx.host.exp(p);
    let (level, into) = level_of(total);
    let shown = display_name(ctx, p);
    ctx.tl("exp", &[plain(&shown), plain(total.to_string()), plain(level.to_string()), plain((exp_at_level(level) - into).to_string())]);
}

fn set_exp(ctx: &mut Ctx, p: Uuid, amount: &str, give: bool) -> CmdResult {
    let s = amount.to_lowercase();
    let mut total = ctx.host.exp(p);
    let mut value: i64 = if s.contains('l') {
        let mut level: i64 = s.replace('l', "").parse().map_err(|_| CmdError::Usage(None))?;
        if give {
            level += level_of(total).0;
        }
        total = 0;
        exp_to_level(level)
    } else {
        let v: i64 = s.parse().map_err(|_| CmdError::Usage(None))?;
        if v > i32::MAX as i64 || v < i32::MIN as i64 {
            return Err(CmdError::Usage(None));
        }
        v
    };
    if give {
        value += total;
    }
    let value = value.clamp(0, i32::MAX as i64);
    ctx.host.set_exp(p, value);
    let shown = display_name(ctx, p);
    ctx.tl("expSet", &[plain(shown), plain(value.to_string())]);
    Ok(())
}

// ----- recipes & condense ---------------------------------------------------------------------------------

/// A recipe's ingredient cell: the item it shows (the first of a list or a tag's name).
fn ingredient_item(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Array(a) => a.first().and_then(ingredient_item),
        Value::Object(o) => o.get("item").or_else(|| o.get("tag").map(|_| &Value::Null)).and_then(Value::as_str).map(str::to_string).or_else(|| o.get("tag").and_then(Value::as_str).map(|t| format!("#{t}"))),
        _ => None,
    }
}

fn recipe_result(r: &Value) -> Option<(String, i32)> {
    let res = r.get("result")?;
    match res {
        Value::String(s) => Some((ns(s), 1)),
        _ => Some((ns(res.get("id").or_else(|| res.get("item"))?.as_str()?), res.get("count").and_then(Value::as_i64).unwrap_or(1) as i32)),
    }
}

/// The crafting cells of a shaped or shapeless recipe, row-major for shaped (None: empty cell), and its width.
fn recipe_cells(r: &Value) -> Option<(Vec<Option<String>>, usize, usize)> {
    let kind = r.get("type").and_then(Value::as_str).map(bare)?;
    match kind {
        "crafting_shaped" => {
            let pattern: Vec<&str> = r.get("pattern")?.as_array()?.iter().filter_map(Value::as_str).collect();
            let key = r.get("key")?;
            let w = pattern.iter().map(|p| p.chars().count()).max().unwrap_or(0);
            let mut cells = Vec::new();
            for row in &pattern {
                let chars: Vec<char> = row.chars().collect();
                for x in 0..w {
                    let c = chars.get(x).copied().unwrap_or(' ');
                    cells.push(if c == ' ' { None } else { key.get(c.to_string()).and_then(ingredient_item) });
                }
            }
            Some((cells, w, pattern.len()))
        }
        "crafting_shapeless" => {
            let cells: Vec<Option<String>> = r.get("ingredients")?.as_array()?.iter().map(ingredient_item).collect();
            let n = cells.len();
            Some((cells, n, 1))
        }
        _ => None,
    }
}

/// getStackOnRecipeMatch: the recipe's (non-empty) ingredients when every one is this item and a shaped
/// recipe is square.
fn recipe_match(r: &Value, item: &Item) -> Option<usize> {
    let (cells, w, h) = recipe_cells(r)?;
    let shaped = r.get("type").and_then(Value::as_str).map(bare) == Some("crafting_shaped");
    if shaped && w != h {
        return None;
    }
    if !same_components(&item.components, &json!({})) {
        return None;
    }
    let inputs: Vec<&String> = cells.iter().flatten().collect();
    inputs.iter().all(|c| ns(c) == item.id).then_some(inputs.len())
}

/// Commandcondense.getCondenseType: (input count, result item, result count).
fn condense_type(recipes: &[Value], item: &Item) -> Option<(i32, String, i32)> {
    let mut best: Option<(i32, String, i32)> = None;
    for r in recipes {
        let Some(n) = recipe_match(r, item) else { continue };
        let Some((result, count)) = recipe_result(r) else { continue };
        if (n == 4 || n == 9) && n as i32 > count && best.as_ref().is_none_or(|b| (n as i32) > b.0) {
            best = Some((n as i32, result, count));
        }
    }
    best
}

fn cmd_condense(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let recipes = ctx.host.recipes();
    let validate = args.is_empty();
    let mut stacks: Vec<Item> = Vec::new();
    let inv = Inv::of(ctx.host, me);
    let mut push = |it: &Item| {
        let mut one = it.clone();
        one.count = 1;
        if !stacks.iter().any(|s| similar(s, &one)) {
            stacks.push(one);
        }
    };
    if validate {
        for (i, s) in inv.slots.iter().enumerate() {
            if let Some(s) = s.as_ref().filter(|_| !is_armor_slot(i)) {
                push(s);
            }
        }
    } else {
        match args[0].to_lowercase().as_str() {
            "hand" => {
                if let Some(h) = held(ctx.host, me).1 {
                    push(&h);
                }
            }
            "inventory" | "invent" | "all" | "blocks" => inv.slots.iter().flatten().for_each(|s| push(s)),
            name => push(&item_stack(ctx.ess, ctx.host, name)?),
        }
        if stacks.is_empty() {
            return Err(CmdError::Msg(ctx.ess.tl("itemSellAir", &[])));
        }
    }
    let mut converted = false;
    for stack in stacks {
        let Some((input, result, out)) = condense_type(&recipes, &stack) else { continue };
        let result_item = Item::new(&result, 1);
        if validate && !recipes.iter().any(|r| recipe_result(r).is_some_and(|(id, _)| id == stack.id) && recipe_match(r, &result_item).is_some()) {
            continue;
        }
        let mut inv = Inv::of(ctx.host, me);
        let have: i32 = (0..41).filter(|i| !is_armor_slot(*i)).filter_map(|i| inv.get(i)).filter(|s| similar(s, &stack)).map(|s| s.count).sum();
        let blocks = have / input;
        let amount = blocks * input;
        if amount > 0 {
            remove_amount(&mut inv, &stack, amount);
            inv.save(ctx.host);
            add_items(ctx.ess, ctx.host, me, vec![Item::new(&result, blocks * out)], true);
            converted = true;
        }
    }
    if converted {
        ctx.tl("itemsConverted", &[]);
        Ok(())
    } else {
        ctx.tl("itemsNotConverted", &[]);
        Err(CmdError::Quiet)
    }
}

fn material_name(ctx: &mut Ctx, id: Option<&str>) -> String {
    match id {
        None => ctx.ess.tl("recipeNothing", &[]),
        Some(id) => bare(id.trim_start_matches('#')).replace('_', " "),
    }
}

/// AdventureUtil.fromChar: a digit's legacy colour as a tag.
fn legacy_color_tag(c: char) -> String {
    match "0123456789abcdef".find(c) {
        Some(i) => format!("<{}>", ess_text::COLOR_NAMES[i]),
        None => String::new(),
    }
}

fn cmd_recipe(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    if args.is_empty() {
        return Err(CmdError::Usage(None));
    }
    let item = if args[0].eq_ignore_ascii_case("hand") {
        let me = ctx.sender.player().ok_or_else(|| tl("consoleCannotUseCommand", vec![]))?;
        held(ctx.host, me).1.unwrap_or_else(|| Item::new("air", 1))
    } else {
        item_stack(ctx.ess, ctx.host, &args[0])?
    };
    let index: i64 = match args.get(1) {
        Some(n) => n.parse::<i64>().map_err(|_| tl("invalidNumber", vec![]))? - 1,
        None => 0,
    };
    let name = mat_spaced(&item);
    let mut recipes: Vec<Value> = Vec::new();
    for r in ctx.host.recipes() {
        if recipe_result(&r).map(|(id, _)| id) != Some(item.id.clone()) {
            continue;
        }
        let kind = r.get("type").and_then(Value::as_str).map(bare).unwrap_or("");
        if kind == "crafting_transmute" {
            let choices = |v: Option<&Value>| match v {
                Some(Value::Array(a)) => a.iter().filter_map(ingredient_item).collect::<Vec<_>>(),
                Some(v) => ingredient_item(v).into_iter().collect(),
                None => Vec::new(),
            };
            for input in choices(r.get("input")) {
                for material in choices(r.get("material")) {
                    recipes.push(json!({"type": "minecraft:crafting_shapeless", "ingredients": [input, material], "result": {"id": item.id}}));
                }
            }
        } else {
            recipes.push(r);
        }
    }
    if recipes.is_empty() {
        return Err(tl("recipeNone", vec![plain(&name)]));
    }
    if index < 0 || index as usize >= recipes.len() {
        return Err(tl("recipeBadIndex", vec![]));
    }
    let r = recipes[index as usize].clone();
    ctx.tl("recipe", &[plain(&name), plain((index + 1).to_string()), plain(recipes.len().to_string())]);
    let kind = r.get("type").and_then(Value::as_str).map(bare).unwrap_or("").to_string();
    if kind == "smelting" {
        let input = r.get("ingredient").and_then(ingredient_item);
        let n = material_name(ctx, input.as_deref());
        ctx.tl("recipeFurnace", &[plain(n)]);
    } else if kind == "crafting_shaped" {
        let (cells, w, h) = recipe_cells(&r).unwrap_or_default();
        // CraftBukkit names a vanilla recipe's cells a..i in order; c..i past the shape are empty.
        let mut by_char: Vec<Option<String>> = cells.iter().map(|c| c.as_deref().map(ns)).collect();
        by_char.resize(9, None);
        let mut colors: Vec<(Option<String>, char)> = Vec::new();
        for c in &by_char {
            if !colors.iter().any(|(k, _)| k == c) {
                let digit = char::from_digit(colors.len() as u32 + 1, 16).unwrap_or('f');
                colors.push((c.clone(), digit));
            }
        }
        let tag = |cell: Option<String>| -> String {
            let d = colors.iter().find(|(k, _)| *k == cell).map_or('1', |(_, d)| *d);
            format!("{}{d}", legacy_color_tag(d))
        };
        for y in 0..3 {
            let cell = |x: usize| if y < h && x < w { cells[y * w + x].as_deref().map(ns) } else { None };
            let row = [tag(cell(0)), tag(cell(1)), tag(cell(2))];
            ctx.tl("recipeGrid", &row.map(Arg::Parsed));
        }
        let mut s = String::new();
        for (k, d) in colors.clone() {
            let n = material_name(ctx, k.as_deref());
            s.push_str(&ctx.ess.tl("recipeGridItem", &[plain(d.to_string()), plain(n)]));
            s.push(' ');
        }
        ctx.tl("recipeWhere", &[Arg::Parsed(s)]);
    } else if kind == "crafting_shapeless" {
        let (cells, _, _) = recipe_cells(&r).unwrap_or_default();
        let mut s = String::new();
        for (i, c) in cells.iter().enumerate() {
            s.push_str(&material_name(ctx, c.as_deref()));
            if i != cells.len() - 1 {
                s.push(',');
            }
            s.push(' ');
        }
        ctx.tl("recipeShapeless", &[plain(s)]);
    }
    if recipes.len() > 1 && args.len() == 1 {
        let label = ctx.label.clone();
        ctx.tl("recipeMore", &[plain(label), plain(&args[0]), plain(&name)]);
    }
    Ok(())
}

// ----- containers & workstations ---------------------------------------------------------------------------

const TAG_DISPOSAL: &str = "items:disposal";
const TAG_ENDER: &str = "items:enderchest:";

fn cmd_disposal(ctx: &mut Ctx, _args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    ctx.tl("openingDisposal", &[]);
    let title = ctx.ess.tl("disposal", &[]);
    ctx.host.open_container(me, &Container { rows: 4, title, items: vec![None; 36], tag: TAG_DISPOSAL.into() });
    Ok(())
}

fn cmd_enderchest(ctx: &mut Ctx, args: &[String]) -> CmdResult {
    let me = ctx.player()?;
    let target = if !args.is_empty() && ctx.has("essentials.enderchest.others") { ctx.online_player(&args[0])? } else { me };
    ctx.host.close_container(me);
    if target == me {
        ctx.host.open_station(me, Station::EnderChest);
        return Ok(());
    }
    let mut items = ctx.host.ender_chest(target);
    items.resize(27, None);
    let title = ess_text::escape_tags(&format!("{}'s Ender Chest", ctx.name_of(target)));
    ctx.host.open_container(me, &Container { rows: 3, title, items, tag: format!("{TAG_ENDER}{}", uuid_string(target)) });
    Ok(())
}

fn on_container_click(ess: &mut Ess, host: &mut dyn Host, player: Uuid, tag: &str, _slot: usize) -> Allow {
    if tag.starts_with(TAG_ENDER) && !ess.has(host, Sender::Player(player), "essentials.enderchest.modify") {
        return Allow::No;
    }
    Allow::Yes
}

fn on_container_close(ess: &mut Ess, host: &mut dyn Host, player: Uuid, tag: &str, items: &[Option<Item>]) {
    let Some(target) = tag.strip_prefix(TAG_ENDER).and_then(parse_uuid) else { return };
    if !ess.has(host, Sender::Player(player), "essentials.enderchest.modify") {
        return;
    }
    let now = host.ender_chest(target);
    for (i, it) in items.iter().enumerate().take(27) {
        if now.get(i) != Some(it) {
            host.set_ender_slot(target, i, it.clone());
        }
    }
}

/// Each tick: newly opened plugin containers get their click policy from the container_click listeners (every
/// raw slot asked once), and the ones players closed reach the container_close listeners.
fn poll_closed_containers(ess: &mut Ess, host: &mut dyn Host) {
    for (player, tag, slots) in host.opened_containers() {
        let allowed: Vec<bool> = (0..slots).map(|s| ess.on_container_click(host, player, &tag, s) == Allow::Yes).collect();
        host.set_container_policy(player, &tag, allowed);
    }
    for (player, tag, items) in host.closed_containers() {
        ess.on_container_close(host, player, &tag, &items);
    }
}

fn station(ctx: &mut Ctx, station: Station, unsupported: bool) -> CmdResult {
    let me = ctx.player()?;
    if !ctx.host.open_station(me, station) && unsupported {
        ctx.tl("unsupportedBrand", &[]);
    }
    Ok(())
}
fn cmd_workbench(ctx: &mut Ctx, _: &[String]) -> CmdResult {
    station(ctx, Station::Crafting, false)
}
fn cmd_anvil(ctx: &mut Ctx, _: &[String]) -> CmdResult {
    station(ctx, Station::Anvil, true)
}
fn cmd_cartographytable(ctx: &mut Ctx, _: &[String]) -> CmdResult {
    station(ctx, Station::Cartography, true)
}
fn cmd_grindstone(ctx: &mut Ctx, _: &[String]) -> CmdResult {
    station(ctx, Station::Grindstone, true)
}
fn cmd_loom(ctx: &mut Ctx, _: &[String]) -> CmdResult {
    station(ctx, Station::Loom, true)
}
fn cmd_smithingtable(ctx: &mut Ctx, _: &[String]) -> CmdResult {
    station(ctx, Station::Smithing, true)
}
fn cmd_stonecutter(ctx: &mut Ctx, _: &[String]) -> CmdResult {
    station(ctx, Station::Stonecutter, true)
}

// ----- tab completion -------------------------------------------------------------------------------------

fn tab_for(name: &str) -> Option<TabFn> {
    match name {
        "kit" | "showkit" | "delkit" | "kitreset" => Some(tab_kits),
        "item" | "itemdb" | "recipe" | "condense" => Some(tab_items),
        "give" => Some(tab_give),
        _ => None,
    }
}
fn players(ctx: &mut Ctx) -> Vec<String> {
    let online = ctx.host.online();
    let mut out = Vec::new();
    for p in online {
        if ctx.ess.can_see(ctx.host, ctx.sender, p) {
            out.push(ctx.name_of(p));
        }
    }
    out
}
fn items_starting(prefix: &str) -> Vec<String> {
    let p = prefix.to_lowercase();
    db::ITEMS.iter().map(|i| i.0).filter(|n| n.starts_with(&p)).take(100).map(str::to_string).collect()
}
fn tab_kits(ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    match args.len() {
        1 => {
            let mut out = Vec::new();
            for k in kit_names(ctx.ess) {
                if ctx.name != "kit" || ctx.has(&format!("essentials.kits.{k}")) {
                    out.push(k);
                }
            }
            out
        }
        2 => players(ctx),
        _ => Vec::new(),
    }
}
fn tab_items(_ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    match args.len() {
        1 => items_starting(&args[0]),
        _ => Vec::new(),
    }
}
fn tab_give(ctx: &mut Ctx, args: &[String]) -> Vec<String> {
    match args.len() {
        1 => players(ctx),
        2 => items_starting(&args[1]),
        3 => vec!["1".into(), "64".into()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ess_testkit::*;
    use serde_json::{Value, json};

    fn it(id: &str, n: i32) -> Item {
        Item::new(id, n)
    }
    fn with(id: &str, n: i32, components: Value) -> Item {
        Item { components, ..Item::new(id, n) }
    }
    fn hold(t: &mut TestServer, p: Uuid, item: Item) {
        let h = t.host.p(p).held;
        t.host.p(p).inventory[h] = Some(item);
    }
    fn held(t: &mut TestServer, p: Uuid) -> Option<Item> {
        let h = t.host.p(p).held;
        t.host.p(p).inventory[h].clone()
    }
    fn count(t: &mut TestServer, p: Uuid, id: &str) -> i32 {
        let id = if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") };
        t.host.p(p).inventory.iter().flatten().filter(|i| i.id == id).map(|i| i.count).sum()
    }
    fn server(config: &str) -> (TestServer, Uuid) {
        let mut t = TestServer::with_config(config);
        let alice = t.join("Alice");
        t.op("Alice");
        (t, alice)
    }
    fn write(t: &TestServer, name: &str, text: &str) {
        std::fs::write(t.ess.dir.join(name), text).unwrap();
    }

    // ----- itemdb ------------------------------------------------------------------------------------------

    #[test]
    fn itemdb_resolves_aliases_and_names() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/itemdb dsword"), ["Item: DIAMOND_SWORD", "Item short names: crystalsword, diamond_sword, diamondsword, dsword"]);
        assert_eq!(t.cmd(alice, "/itemdb minecraft:stone")[0], "Item: STONE");
        assert_eq!(t.cmd(alice, "/itemdb nosuchthing"), ["Error: Unknown item name: nosuchthing."]);
        hold(&mut t, alice, with("diamond_sword", 1, json!({"damage": 61})));
        let lines = t.cmd(alice, "/dura");
        assert_eq!(lines[..2], ["Item: DIAMOND_SWORD", "This tool has 1501 uses left."]);
        // custom_items.yml aliases resolve too, and /itemdb lists them.
        write(&t, "custom_items.yml", "aliases:\n  rock: stone\n");
        assert_eq!(t.cmd(alice, "/itemdb rock")[0], "Item: STONE");
        assert!(t.cmd(alice, "/itemdb stone")[1].contains("rock"));
        // Potions and spawners keep their data.
        let stack = item_stack(&mut t.ess, &mut t.host, "potionofswiftnessii").unwrap();
        assert_eq!(stack.components["potion_contents"]["potion"], "minecraft:strong_swiftness");
        let spawner = item_stack(&mut t.ess, &mut t.host, "pigspawner").unwrap();
        assert_eq!(spawner.id, "minecraft:spawner");
        assert_eq!(spawner.components["block_entity_data"]["SpawnData"]["entity"]["id"], "minecraft:pig");
    }

    // ----- item / give / more -----------------------------------------------------------------------------

    #[test]
    fn item_gives_stacks_and_meta() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/item stone 10"), ["Giving 10 of stone"]);
        assert_eq!(t.host.p(alice).inventory[0], Some(it("stone", 10)));
        assert_eq!(t.cmd(alice, "/i dirt"), ["Giving 64 of dirt"]);
        assert_eq!(count(&mut t, alice, "dirt"), 64);
        t.cmd(alice, "/item dsword 1 sharpness:9 knockback name:&4Blade lore:Line_one|Line_two");
        let sword = t.host.p(alice).inventory[2].clone().unwrap();
        assert_eq!(sword.id, "minecraft:diamond_sword");
        assert_eq!(sword.components["enchantments"], json!({"minecraft:sharpness": 5, "minecraft:knockback": 2}));
        assert_eq!(component_plain(&sword.components["custom_name"]), "Blade");
        assert_eq!(sword.components["lore"].as_array().unwrap().len(), 2);
        assert_eq!(component_plain(&sword.components["lore"][1]), "Line two");
        assert_eq!(t.cmd(alice, "/item nosuch"), ["Error: Unknown item name: nosuch."]);
        assert!(t.cmd(alice, "/item")[0].starts_with("Command Help"));
    }

    #[test]
    fn item_splits_stacks_and_honours_oversized() {
        let (mut t, alice) = server("oversized-stacksize: 128");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.item]\n  big:\n    permissions: [essentials.item, essentials.oversizedstacks]\nusers:\n  bob:\n    groups: [big]\n");
        let carl = t.join("Carl");
        assert_eq!(t.cmd(carl, "/item stone 100"), ["Giving 100 of stone"]);
        assert_eq!(t.host.p(carl).inventory[0], Some(it("stone", 64)));
        assert_eq!(t.host.p(carl).inventory[1], Some(it("stone", 36)));
        let bob = t.join("Bob");
        t.cmd(bob, "/item stone 100");
        assert_eq!(t.host.p(bob).inventory[0], Some(it("stone", 100)));
        let _ = alice;
    }

    #[test]
    fn item_spawn_blacklist_and_permission_based_spawn() {
        let (mut t, alice) = server("item-spawn-blacklist: tnt,bedrock");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.item]\n");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/item tnt"), ["Error: You are not allowed to spawn the item tnt."]);
        assert_eq!(t.cmd(bob, "/item stone 1"), ["Giving 1 of stone"]);
        let _ = alice;
        let (mut t, _) = server("permission-based-item-spawn: true");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.item, essentials.itemspawn.item-stone]\n");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/item stone 1"), ["Giving 1 of stone"]);
        assert_eq!(t.cmd(bob, "/item dirt 1"), ["Error: You are not allowed to spawn the item dirt."]);
    }

    #[test]
    fn give_to_players_and_overflow() {
        let (mut t, alice) = server("");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(alice, "/give Bob diamond 5"), ["Giving 5 of diamond to Bob."]);
        assert_eq!(count(&mut t, bob, "diamond"), 5);
        assert_eq!(t.console("/give bob emerald 3"), ["Giving 3 of emerald to Bob."]);
        assert_eq!(count(&mut t, bob, "emerald"), 3);
        for s in 0..41 {
            t.host.p(bob).inventory[s] = Some(it("dirt", 64));
        }
        assert_eq!(t.cmd(alice, "/give Bob stone 5"), ["Giving 5 of stone to Bob.", "Not enough space, 5 stone was lost."]);
        assert!(t.host.dropped.is_empty());
        assert_eq!(t.cmd(alice, "/give Nobody stone 5"), ["Error: Player not found."]);
        let (mut t, alice) = server("drop-items-if-full: true");
        let bob = t.join("Bob");
        for s in 0..41 {
            t.host.p(bob).inventory[s] = Some(it("dirt", 64));
        }
        t.cmd(alice, "/give Bob stone 5");
        assert_eq!(t.host.dropped.len(), 1);
        assert_eq!(t.host.dropped[0].1, it("stone", 5));
    }

    #[test]
    fn more_fills_the_stack() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/more"), ["Error: You are not allowed to spawn the item Air."]);
        hold(&mut t, alice, it("stone", 10));
        assert_eq!(t.cmd(alice, "/more 5"), Vec::<String>::new());
        assert_eq!(held(&mut t, alice).unwrap().count, 15);
        // An operator has essentials.oversizedstacks (oversized-stacksize: 64).
        assert_eq!(t.cmd(alice, "/more 60"), ["Your stack has been set to its maximum size, 64."]);
        assert_eq!(t.cmd(alice, "/more"), ["Error: You already have a full stack."]);
        hold(&mut t, alice, it("ender_pearl", 1));
        t.cmd(alice, "/more");
        assert_eq!(held(&mut t, alice).unwrap().count, 64);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.more]\n");
        let bob = t.join("Bob");
        hold(&mut t, bob, it("ender_pearl", 10));
        assert_eq!(t.cmd(bob, "/more 10"), ["Your stack has been set to its default size, 16."]);
        assert_eq!(held(&mut t, bob).unwrap().count, 16);
    }

    // ----- kits --------------------------------------------------------------------------------------------

    #[test]
    fn kit_gives_items_and_honours_delays() {
        let mut t = TestServer::new();
        let bob = t.join("Bob");
        assert_eq!(t.cmd(bob, "/kit"), ["Kits: Tools"]);
        assert_eq!(t.cmd(bob, "/kit tools"), ["Received kit tools."]);
        for tool in ["stone_sword", "stone_shovel", "stone_pickaxe", "stone_axe"] {
            assert_eq!(count(&mut t, bob, tool), 1, "{tool}");
        }
        assert_eq!(t.cmd(bob, "/kit tools"), ["You can't use that kit again for another 10 seconds."]);
        assert!(t.userdata(bob)["timestamps"]["kits"]["tools"].as_i64().is_some());
        t.host.now += 11_000;
        assert_eq!(t.cmd(bob, "/kit tools"), ["Received kit tools."]);
        assert_eq!(t.cmd(bob, "/kit dtools"), ["Error: You need the essentials.kits.dtools permission to use that kit."]);
        assert_eq!(t.cmd(bob, "/kit nope"), ["Error: That kit does not exist."]);
    }

    #[test]
    fn kit_meta_commands_and_others() {
        let (mut t, alice) = server("");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(alice, "/kit dtools Bob"), ["Giving kit dtools to Bob."]);
        assert_eq!(t.said(bob), ["Received kit dtools."]);
        let inv = t.host.p(bob).inventory.clone();
        let pick = inv.iter().flatten().find(|i| i.id == "minecraft:diamond_pickaxe").unwrap();
        assert_eq!(pick.components["enchantments"], json!({"minecraft:efficiency": 1, "minecraft:unbreaking": 1, "minecraft:fortune": 1}));
        assert_eq!(component_plain(&pick.components["custom_name"]), "Gigadrill");
        let helm = inv.iter().flatten().find(|i| i.id == "minecraft:leather_helmet").unwrap();
        assert_eq!(helm.components["dyed_color"], json!(0xFFFFFF));
        let axe = inv.iter().flatten().find(|i| i.id == "minecraft:diamond_axe").unwrap();
        assert_eq!(axe.components["damage"], json!(780));
        assert_eq!(t.host.console_commands, ["broadcast Bob just got some fancy tools!"]);
        // The console gives kits to a player too.
        assert_eq!(t.console("/kit notch Bob"), ["Giving kit notch to Bob."]);
        let head = t.host.p(bob).inventory.iter().flatten().find(|i| i.id == "minecraft:player_head").cloned().unwrap();
        assert_eq!(head.components["profile"]["name"], "Notch");
        // Every kit for an operator; the list marks the ones on cooldown.
        assert_eq!(t.cmd(alice, "/kits"), ["Kits: Color Dtools Firework Notch Tools"]);
        // The public API for other areas.
        let carl = t.join("Carl");
        assert_eq!(give_kit(&mut t.ess, &mut t.host, carl, "tools"), Ok(true));
        assert_eq!(count(&mut t, carl, "stone_axe"), 1);
    }

    #[test]
    fn kit_fireworks_and_books() {
        let (mut t, alice) = server("");
        t.cmd(alice, "/kit firework");
        let rockets: Vec<Item> = t.host.p(alice).inventory.iter().flatten().cloned().collect();
        assert_eq!(rockets[0].components["fireworks"]["explosions"][0]["shape"], "creeper");
        assert_eq!(rockets[0].components["fireworks"]["flight_duration"], 1);
        assert_eq!(rockets[2].count, 2);
        assert_eq!(rockets[2].components["fireworks"]["explosions"].as_array().unwrap().len(), 3);
        assert_eq!(rockets[1].components["fireworks"]["explosions"][0], json!({"shape": "star", "colors": [0xDECF2A, 0xEB8844], "fade_colors": [0x253192], "has_trail": true, "has_twinkle": true}));
        t.cmd(alice, "/kit color");
        let book = t.host.p(alice).inventory.iter().flatten().find(|i| i.id == "minecraft:written_book").cloned().unwrap();
        assert_eq!(book.components["written_book_content"]["author"], "KHobbits");
        assert!(!book.components["written_book_content"]["pages"].as_array().unwrap().is_empty());
    }

    #[test]
    fn kit_options_slots_once_full_inventory_and_auto_equip() {
        let (mut t, alice) = server("kit-auto-equip: true");
        write(&t, "kits.yml", "kits:\n  once:\n    delay: -1\n    items:\n    - slot:5 stone 1\n    - diamond_helmet 1\n    - $50\n  big:\n    delay: 0\n    items:\n    - dirt 64\n    - dirt 64\n");
        assert_eq!(t.cmd(alice, "/kit once"), ["Received kit once."]);
        assert_eq!(t.host.p(alice).inventory[5], Some(it("stone", 1)));
        assert_eq!(t.host.p(alice).inventory[39], Some(it("diamond_helmet", 1)));
        assert_eq!(t.cmd(alice, "/kit once"), ["You can't use that kit again."]);
        assert_eq!(t.cmd(alice, "/kitreset once"), ["Reset cooldown for kit once."]);
        assert_eq!(t.cmd(alice, "/kit once"), ["Received kit once."]);
        for s in 0..36 {
            t.host.p(alice).inventory[s] = Some(it("cobblestone", 64));
        }
        t.host.p(alice).inventory[40] = None;
        assert_eq!(t.cmd(alice, "/kit big"), ["There is not enough room in your inventory for that kit."]);
        let (mut t, alice) = server("drop-items-if-full: true\nskip-used-one-time-kits-from-kit-list: true");
        write(&t, "kits.yml", "kits:\n  big:\n    delay: -1\n    items:\n    - dirt 64\n    - dirt 64\n");
        for s in 0..36 {
            t.host.p(alice).inventory[s] = Some(it("cobblestone", 64));
        }
        assert_eq!(t.cmd(alice, "/kit big"), ["Your inventory was full, placing kit on the floor.", "Received kit big."]);
        assert_eq!(t.host.p(alice).inventory[40], Some(it("dirt", 64)));
        assert_eq!(t.host.dropped.len(), 1);
        assert_eq!(t.cmd(alice, "/kit"), ["There are no kits available yet."]);
    }

    #[test]
    fn showkit_createkit_delkit() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/showkit tools"), ["Kit tools contains:", "- stonesword 1", "- stoneshovel 1", "- stonepickaxe 1", "- stoneaxe 1"]);
        t.host.p(alice).inventory[0] = Some(it("stone", 5));
        t.host.p(alice).inventory[3] = Some(with("diamond_sword", 1, json!({"enchantments": {"minecraft:sharpness": 2}, "custom_name": "Big Blade"})));
        assert_eq!(t.cmd(alice, "/createkit starter 60"), ["Created kit starter with 2 entries and delay 60"]);
        let kits = t.ess.read_yaml("kits.yml");
        assert_eq!(kits["kits"]["starter"]["items"], json!(["slot:0 stone 5", "slot:3 diamond_sword 1 name:Big_Blade sharpness:2"]));
        assert_eq!(kits["kits"]["starter"]["delay"], json!(60));
        t.host.p(alice).inventory = vec![None; 41];
        assert_eq!(t.cmd(alice, "/kit starter"), ["Received kit starter."]);
        assert_eq!(t.host.p(alice).inventory[3].as_ref().unwrap().components["enchantments"], json!({"minecraft:sharpness": 2}));
        assert_eq!(t.cmd(alice, "/delkit starter"), ["Kit starter has been removed."]);
        assert!(t.ess.read_yaml("kits.yml")["kits"].get("starter").is_none());
        assert_eq!(t.cmd(alice, "/delkit starter"), ["Error: That kit does not exist."]);
    }

    // ----- inventories -------------------------------------------------------------------------------------

    #[test]
    fn clearinventory_variants_and_confirmation() {
        let (mut t, alice) = server("");
        let bob = t.join("Bob");
        t.host.p(alice).inventory[0] = Some(it("stone", 5));
        t.host.p(alice).inventory[39] = Some(it("diamond_helmet", 1));
        t.host.p(alice).inventory[40] = Some(it("torch", 5));
        assert_eq!(t.cmd(alice, "/ci"), ["Cleared all inventory items from Alice."]);
        assert_eq!(t.host.p(alice).inventory[0], None);
        assert_eq!(t.host.p(alice).inventory[40], None);
        assert_eq!(t.host.p(alice).inventory[39], Some(it("diamond_helmet", 1)));
        assert_eq!(t.cmd(alice, "/ci Alice **"), ["Cleared all inventory items and armor from Alice."]);
        assert_eq!(t.host.p(alice).inventory[39], None);
        t.host.p(bob).inventory[0] = Some(it("stone", 10));
        t.host.p(bob).inventory[1] = Some(it("stone", 10));
        assert_eq!(t.cmd(alice, "/ci Bob stone 15"), ["Removed 15 of stone from Bob."]);
        assert_eq!(count(&mut t, bob, "stone"), 5);
        assert_eq!(t.cmd(alice, "/ci Bob stone 50"), ["Player Bob does not have 50 of stone."]);
        assert_eq!(t.cmd(alice, "/ci Bob stone"), ["Removed 5 of stone from Bob."]);
        assert_eq!(t.cmd(alice, "/clearinventoryconfirmtoggle"), ["You will now be prompted to confirm inventory clears."]);
        assert_eq!(t.userdata(alice)["confirm-clear"], json!(true));
        t.host.p(alice).inventory[0] = Some(it("stone", 5));
        assert_eq!(t.cmd(alice, "/ci"), ["To CONFIRM inventory clear, please repeat command: /ci"]);
        assert_eq!(count(&mut t, alice, "stone"), 5);
        assert_eq!(t.cmd(alice, "/ci"), ["Cleared all inventory items from Alice."]);
        assert_eq!(t.cmd(alice, "/clearconfirmoff"), ["You will no longer be prompted to confirm inventory clears."]);
        // A regular player only clears themselves.
        t.host.p(alice).inventory[0] = Some(it("stone", 5));
        // EssentialsX reads "Alice" as an item list then: nothing is cleared, nothing said.
        assert_eq!(t.cmd(bob, "/ci Alice"), Vec::<String>::new());
        assert_eq!(count(&mut t, alice, "stone"), 5);
    }

    #[test]
    fn hat_wears_and_removes() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/hat"), ["You must have something to wear in your hand."]);
        hold(&mut t, alice, it("diamond_sword", 1));
        assert_eq!(t.cmd(alice, "/hat"), ["You cannot use this item as a hat!"]);
        hold(&mut t, alice, it("dirt", 1));
        assert_eq!(t.cmd(alice, "/hat"), ["Enjoy your new hat!"]);
        assert_eq!(t.host.p(alice).inventory[39], Some(it("dirt", 1)));
        assert_eq!(held(&mut t, alice), None);
        assert_eq!(t.cmd(alice, "/hat remove"), ["Your hat has been removed."]);
        assert_eq!(t.host.p(alice).inventory[39], None);
        assert_eq!(count(&mut t, alice, "dirt"), 1);
        assert_eq!(t.cmd(alice, "/hat off"), ["You are not wearing a hat."]);
        t.host.p(alice).inventory[39] = Some(with("carved_pumpkin", 1, json!({"enchantments": {"minecraft:binding_curse": 1}})));
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.hat]\n");
        let bob = t.join("Bob");
        t.host.p(bob).inventory[39] = t.host.p(alice).inventory[39].clone();
        assert_eq!(t.cmd(bob, "/hat remove"), ["You cannot remove a hat with the curse of binding!"]);
    }

    #[test]
    fn repair_hand_and_all() {
        let (mut t, alice) = server("");
        hold(&mut t, alice, it("stone", 1));
        assert_eq!(t.cmd(alice, "/repair"), ["Error: This item cannot be repaired."]);
        hold(&mut t, alice, with("diamond_sword", 1, json!({"damage": 100})));
        assert_eq!(t.cmd(alice, "/fix"), ["You have successfully repaired your: diamond sword."]);
        assert_eq!(held(&mut t, alice).unwrap().components.get("damage"), None);
        t.host.p(alice).inventory[3] = Some(with("bow", 1, json!({"damage": 5})));
        t.host.p(alice).inventory[38] = Some(with("diamond_chestplate", 1, json!({"damage": 5})));
        assert_eq!(t.cmd(alice, "/repair all"), ["You have successfully repaired your: bow, diamond chestplate."]);
        assert_eq!(t.cmd(alice, "/repair all"), ["Error: There were no items that needed repairing."]);
        let (mut t, alice) = server("repair-enchanted: false");
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.repair]\n");
        let bob = t.join("Bob");
        hold(&mut t, bob, with("diamond_sword", 1, json!({"damage": 100, "enchantments": {"minecraft:sharpness": 1}})));
        assert_eq!(t.cmd(bob, "/repair"), ["Error: You are not allowed to repair enchanted items."]);
        let _ = alice;
    }

    #[test]
    fn enchant_applies_removes_and_checks() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/enchant sharpness"), ["Error: You have nothing in your hand."]);
        hold(&mut t, alice, it("diamond_sword", 1));
        assert_eq!(t.cmd(alice, "/enchant sharpness 3"), ["The enchantment sharpness has been applied to your item in hand."]);
        assert_eq!(held(&mut t, alice).unwrap().components["enchantments"], json!({"minecraft:sharpness": 3}));
        assert_eq!(t.cmd(alice, "/enchant sharp 0"), ["The enchantment sharpness has been removed from your item in hand."]);
        assert_eq!(held(&mut t, alice).unwrap().components.get("enchantments"), None);
        assert_eq!(t.cmd(alice, "/enchant sharpness 9"), ["Error: Enchantment sharpness: Enchantment level is either too low or too high (given 9, bounds are 1 to 5)"]);
        t.host.unenchantable.push(("minecraft:diamond_sword".into(), "minecraft:protection".into()));
        assert_eq!(t.cmd(alice, "/enchant protection 1"), ["Error: Enchantment protection: Specified enchantment cannot be applied to this itemstack"]);
        let usage = t.cmd(alice, "/enchant");
        assert!(usage.last().unwrap().starts_with("Enchantments: "), "{usage:?}");
        assert!(usage.last().unwrap().contains("sharpness"));
        let names: Vec<&str> = usage.last().unwrap()["Enchantments: ".len()..].split(", ").collect();
        assert!(names.contains(&"sharpness") && !names.contains(&"protection"), "{names:?}");
        hold(&mut t, alice, it("enchanted_book", 1));
        t.cmd(alice, "/enchant mending");
        assert_eq!(held(&mut t, alice).unwrap().components["stored_enchantments"], json!({"minecraft:mending": 1}));
        let (mut t, alice) = server("unsafe-enchantments: true");
        hold(&mut t, alice, it("diamond_sword", 1));
        t.cmd(alice, "/enchant sharpness 10");
        assert_eq!(held(&mut t, alice).unwrap().components["enchantments"], json!({"minecraft:sharpness": 10}));
    }

    #[test]
    fn itemname_and_itemlore() {
        let (mut t, alice) = server("max-itemlore-lines: 2");
        assert_eq!(t.cmd(alice, "/itemname Hi"), ["You need to hold an item to rename it."]);
        hold(&mut t, alice, it("diamond_sword", 1));
        assert_eq!(t.cmd(alice, "/itemname &cCool Sword"), ["You have renamed your held item to \"Cool Sword\"."]);
        let name = held(&mut t, alice).unwrap().components["custom_name"].clone();
        assert_eq!(component_plain(&name), "Cool Sword");
        assert_eq!(name["extra"][0]["color"], "red");
        assert_eq!(t.cmd(alice, "/iname"), ["You have cleared this item's name."]);
        assert_eq!(held(&mut t, alice).unwrap().components.get("custom_name"), None);
        assert_eq!(t.cmd(alice, "/lore add First line"), ["You have added \"First line\" to your held item's lore."]);
        t.cmd(alice, "/lore add Second");
        assert_eq!(t.cmd(alice, "/lore set 2 Changed"), ["You have set line 2 of your held item's lore to \"Changed\"."]);
        assert_eq!(component_plain(&held(&mut t, alice).unwrap().components["lore"][1]), "Changed");
        assert_eq!(t.cmd(alice, "/lore set 5 Nope"), ["Error: Your held item does not have lore text on line 5."]);
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.itemlore]\n");
        let bob = t.join("Bob");
        let lored = held(&mut t, alice).unwrap();
        hold(&mut t, bob, lored);
        assert_eq!(t.cmd(bob, "/lore add Third"), ["Error: You cannot add any more lore lines to this item."]);
        assert_eq!(t.cmd(alice, "/lore clear"), ["You have cleared this item's lore."]);
        assert_eq!(t.cmd(alice, "/lore set 1 x"), ["Error: Your held item does not have any lore text."]);
    }

    #[test]
    fn book_sign_and_unsign() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/book"), ["Error: You are not holding a writable book."]);
        hold(&mut t, alice, with("writable_book", 1, json!({"writable_book_content": {"pages": [{"raw": "Hello"}]}})));
        assert_eq!(t.cmd(alice, "/book"), ["This book is now locked."]);
        let book = held(&mut t, alice).unwrap();
        assert_eq!(book.id, "minecraft:written_book");
        assert_eq!(book.components["written_book_content"]["pages"][0]["raw"], "Hello");
        assert_eq!(t.cmd(alice, "/book title My Story"), ["Title of the book set to My Story."]);
        assert_eq!(held(&mut t, alice).unwrap().components["written_book_content"]["title"]["raw"], "My Story");
        assert_eq!(t.cmd(alice, "/book author Alice"), ["Author of the book set to Alice."]);
        assert_eq!(t.cmd(alice, "/book"), ["You may now edit the contents of this book."]);
        let book = held(&mut t, alice).unwrap();
        assert_eq!(book.id, "minecraft:writable_book");
        assert_eq!(book.components["writable_book_content"]["pages"][0]["raw"], "Hello");
        // Someone else's book stays locked without essentials.book.others.
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.book]\n");
        let bob = t.join("Bob");
        hold(&mut t, bob, with("written_book", 1, json!({"written_book_content": {"title": {"raw": "T"}, "author": "Alice", "pages": []}})));
        assert_eq!(t.cmd(bob, "/book"), ["Error: You cannot unlock this book."]);
        assert_eq!(t.cmd(bob, "/book title X"), ["Error: You cannot change the title of this book."]);
    }

    #[test]
    fn skull_gives_heads() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/skull Notch"), ["You have been given the skull of Notch."]);
        let head = t.host.p(alice).inventory[0].clone().unwrap();
        assert_eq!(head.id, "minecraft:player_head");
        assert_eq!(head.components["profile"]["name"], "Notch");
        assert_eq!(component_plain(&head.components["custom_name"]), "Skull of Notch");
        assert_eq!(t.cmd(alice, "/skull bad-name!"), ["Error: Player names can only contain letters, numbers and underscores."]);
        hold(&mut t, alice, it("player_head", 1));
        assert_eq!(t.cmd(alice, "/skull Jeb_"), ["Skull changed to Jeb_."]);
        assert_eq!(held(&mut t, alice).unwrap().components["profile"]["name"], "Jeb_");
        let bob = t.join("Bob");
        // EssentialsX checks essentials.skull.others on the receiver: Bob, not an operator, gets Alice's skull.
        assert_eq!(t.cmd(alice, "/skull Notch Bob"), ["You have given Bob the skull of Alice."]);
        t.said(bob);
        t.op("Bob");
        assert_eq!(t.cmd(alice, "/skull Notch Bob"), ["You have given Bob the skull of Notch."]);
        assert_eq!(t.said(bob), ["You have been given the skull of Notch."]);
    }

    #[test]
    fn firework_effects_power_and_clear() {
        let (mut t, alice) = server("");
        hold(&mut t, alice, it("stone", 1));
        assert_eq!(t.cmd(alice, "/firework clear"), ["Error: You must be holding a firework to add effects."]);
        hold(&mut t, alice, it("firework_rocket", 3));
        t.cmd(alice, "/firework color:red,#00ff00 fade:blue shape:star effect:trail,twinkle");
        let rocket = held(&mut t, alice).unwrap();
        assert_eq!(rocket.components["fireworks"]["explosions"][0], json!({"shape": "star", "colors": [0xB3312C, 0x00FF00], "fade_colors": [0x253192], "has_trail": true, "has_twinkle": true}));
        t.cmd(alice, "/firework power 2");
        assert_eq!(held(&mut t, alice).unwrap().components["fireworks"]["flight_duration"], 2);
        assert_eq!(t.cmd(alice, "/firework clear"), ["Removed all effects from held stack."]);
        assert_eq!(held(&mut t, alice).unwrap().components["fireworks"]["explosions"], json!([]));
        let lines = t.cmd(alice, "/firework shape:star");
        assert_eq!(lines.last().unwrap(), "Error: Invalid firework charge parameters inserted, must set a color first.");
        t.cmd(alice, "/firework fire 2");
        assert_eq!(t.host.entities.len(), 2);
        assert_eq!(t.host.entities[0].0, "minecraft:firework_rocket");
    }

    #[test]
    fn potion_effects() {
        let (mut t, alice) = server("");
        hold(&mut t, alice, it("stone", 1));
        assert_eq!(t.cmd(alice, "/potion clear"), ["Error: You must be holding a potion to apply effects to it."]);
        hold(&mut t, alice, with("potion", 1, json!({"potion_contents": {"potion": "minecraft:water"}})));
        assert_eq!(t.cmd(alice, "/potion effect:speed power:2 duration:60"), Vec::<String>::new());
        let potion = held(&mut t, alice).unwrap();
        assert_eq!(potion.components["potion_contents"]["custom_effects"], json!([{"id": "minecraft:speed", "amplifier": 1, "duration": 1200}]));
        assert_eq!(potion.components["potion_contents"]["potion"], "minecraft:water");
        t.cmd(alice, "/potion apply");
        assert_eq!(t.host.console_commands.last().unwrap(), "effect give Alice minecraft:speed 60 1");
        t.cmd(alice, "/potion clear");
        assert_eq!(held(&mut t, alice).unwrap().components["potion_contents"].get("custom_effects"), None);
        assert_eq!(t.cmd(alice, "/potion effect:nonsense power:1 duration:1"), ["Error: Invalid potion meta: nonsense."]);
        assert!(t.cmd(alice, "/potion")[0..].iter().any(|l| l.starts_with("Potions: ")));
    }

    #[test]
    fn exp_show_set_give_take() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/exp"), ["Alice has 0 exp (level 0) and needs 7 more exp to level up."]);
        assert_eq!(t.cmd(alice, "/exp set 100"), ["Alice now has 100 exp."]);
        assert_eq!(t.host.p(alice).exp, 100);
        assert_eq!(t.cmd(alice, "/xp give 5"), ["Alice now has 105 exp."]);
        assert_eq!(t.cmd(alice, "/xp take 200"), ["Alice now has 0 exp."]);
        assert_eq!(t.cmd(alice, "/xp give 10l"), ["Alice now has 160 exp."]);
        assert_eq!(t.cmd(alice, "/xp show"), ["Alice has 160 exp (level 10) and needs 27 more exp to level up."]);
        let bob = t.join("Bob");
        assert_eq!(t.cmd(alice, "/xp set Bob 7"), ["Bob now has 7 exp."]);
        assert_eq!(t.host.p(bob).exp, 7);
        assert_eq!(t.cmd(bob, "/xp set 5"), ["You do not have access to that command."]);
    }

    #[test]
    fn condense_and_recipes() {
        let (mut t, alice) = server("");
        t.host.p(alice).inventory[0] = Some(it("iron_ingot", 20));
        t.host.p(alice).inventory[1] = Some(it("stick", 5));
        assert_eq!(t.cmd(alice, "/condense"), ["Converted all items into blocks."]);
        assert_eq!(count(&mut t, alice, "iron_ingot"), 2);
        assert_eq!(count(&mut t, alice, "iron_block"), 2);
        assert_eq!(count(&mut t, alice, "stick"), 5);
        assert_eq!(t.cmd(alice, "/condense"), ["You have no items that can be converted into blocks."]);
        t.host.p(alice).inventory[5] = Some(it("iron_nugget", 18));
        assert_eq!(t.cmd(alice, "/condense ironnugget"), ["Converted all items into blocks."]);
        assert_eq!(count(&mut t, alice, "iron_ingot"), 4);
        assert_eq!(t.console("/recipe iron_block"), ["Recipe for iron block (1 of 1)", "1X | 1X | 1X", "1X | 1X | 1X", "1X | 1X | 1X", "Where: 1X is iron ingot "]);
        assert_eq!(t.console("/recipe iron_ingot"), ["Recipe for iron ingot (1 of 3)", "Combine iron block ", "Type /recipe iron_ingot <number> to see other recipes for iron ingot."]);
        assert_eq!(t.console("/recipe iron_ingot 3"), ["Recipe for iron ingot (3 of 3)", "Smelt: iron ore."]);
        assert_eq!(t.console("/recipe iron_ingot 9"), ["Error: There is no recipe by that number."]);
        assert_eq!(t.console("/recipe bedrock"), ["Error: No recipes exist for bedrock."]);
        assert_eq!(t.console("/recipe torch"), ["Recipe for torch (1 of 1)", "1X | 3X | 3X", "2X | 3X | 3X", "3X | 3X | 3X", "Where: 1X is coal 2X is stick 3X is nothing "]);
    }

    // ----- containers & stations ---------------------------------------------------------------------------

    #[test]
    fn disposal_enderchest_and_workstations() {
        let (mut t, alice) = server("");
        let bob = t.join("Bob");
        assert_eq!(t.cmd(alice, "/trash"), ["Opening disposal menu..."]);
        let open = t.host.p(alice).open.clone().unwrap();
        assert_eq!((open.rows, open.title.as_str(), open.tag.as_str()), (4, "Disposal", "items:disposal"));
        t.ess.on_container_close(&mut t.host, alice, "items:disposal", &[Some(it("dirt", 5))]);
        assert_eq!(count(&mut t, alice, "dirt"), 0);
        assert_eq!(t.cmd(alice, "/ec"), Vec::<String>::new());
        assert_eq!(t.host.p(alice).station, Some(Station::EnderChest));
        t.host.p(bob).ender[0] = Some(it("diamond", 3));
        t.cmd(alice, "/enderchest Bob");
        let open = t.host.p(alice).open.clone().unwrap();
        assert_eq!(open.rows, 3);
        assert_eq!(open.items[0], Some(it("diamond", 3)));
        assert_eq!(t.ess.on_container_click(&mut t.host, alice, &open.tag, 0), Allow::Yes);
        let mut items = open.items.clone();
        items[0] = None;
        items[1] = Some(it("stone", 1));
        // Closing goes through the host's poll of closed containers each tick.
        t.host.closed.push((alice, open.tag.clone(), items));
        t.tick(1);
        assert_eq!(t.host.p(bob).ender[0], None);
        assert_eq!(t.host.p(bob).ender[1], Some(it("stone", 1)));
        // Without essentials.enderchest.modify the view is read-only.
        t.permissions("groups:\n  default:\n    default: true\n    permissions: [essentials.enderchest, essentials.enderchest.others]\n");
        t.cmd(bob, "/ec Alice");
        let tag = t.host.p(bob).open.clone().unwrap().tag;
        assert_eq!(t.ess.on_container_click(&mut t.host, bob, &tag, 0), Allow::No);
        // The host gets every raw slot's answer once, on the tick after the open.
        t.tick(1);
        let policy = t.host.policies.get(&(bob, tag.clone())).cloned().unwrap();
        assert_eq!(policy.len(), 27 + 36);
        assert!(policy.iter().all(|a| !a));
        assert!(t.host.policies.get(&(alice, open.tag.clone())).unwrap().iter().all(|a| *a));
        for (cmd, station) in [("/workbench", Station::Crafting), ("/anvil", Station::Anvil), ("/cartographytable", Station::Cartography), ("/grindstone", Station::Grindstone), ("/loom", Station::Loom), ("/smithingtable", Station::Smithing), ("/stonecutter", Station::Stonecutter)] {
            t.cmd(alice, cmd);
            assert_eq!(t.host.p(alice).station, Some(station), "{cmd}");
        }
    }

    // ----- unlimited & powertools ----------------------------------------------------------------------------

    #[test]
    fn unlimited_toggles_and_refills() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/unlimited stone"), ["Giving unlimited amount of stone to Alice."]);
        assert_eq!(count(&mut t, alice, "stone"), 2);
        assert_eq!(t.userdata(alice)["unlimited"], json!(["STONE"]));
        assert_eq!(t.cmd(alice, "/ul list"), ["Unlimited items: stone"]);
        // A placed block comes back in survival.
        t.host.p(alice).held = 0;
        on_use_item(&mut t.ess, &mut t.host, alice, true);
        t.host.p(alice).inventory[0].as_mut().unwrap().count = 1;
        t.tick(2);
        assert_eq!(t.host.p(alice).inventory[0], Some(it("stone", 2)));
        assert_eq!(t.cmd(alice, "/unlimited stone"), ["Disabled unlimited placing of stone for Alice."]);
        on_use_item(&mut t.ess, &mut t.host, alice, true);
        t.host.p(alice).inventory[0].as_mut().unwrap().count = 1;
        t.tick(2);
        assert_eq!(t.host.p(alice).inventory[0], Some(it("stone", 1)));
        let bob = t.join("Bob");
        assert_eq!(t.cmd(alice, "/unlimited dirt Bob"), ["Giving unlimited amount of dirt to Bob."]);
        assert_eq!(t.said(bob), ["Giving unlimited amount of dirt to Bob."]);
        assert_eq!(t.cmd(alice, "/unlimited clear Bob"), ["Disabled unlimited placing of dirt for Bob."]);
        assert_eq!(t.cmd(alice, "/ul list"), ["Unlimited items: none"]);
    }

    #[test]
    fn powertools_assign_list_toggle_and_fire() {
        let (mut t, alice) = server("");
        assert_eq!(t.cmd(alice, "/pt say hi"), ["Error: Command can't be attached to air."]);
        hold(&mut t, alice, it("stick", 1));
        assert_eq!(t.cmd(alice, "/powertool say hi"), ["say hi command assigned to stick."]);
        assert_eq!(t.cmd(alice, "/pt a:me waves"), ["say hi, me waves command assigned to stick."]);
        assert_eq!(t.cmd(alice, "/pt a:me waves"), ["Error: Command me waves is already assigned to stick."]);
        assert_eq!(t.userdata(alice)["powertools"]["stick"], json!(["say hi", "me waves"]));
        assert_eq!(t.cmd(alice, "/pt l:"), ["Item stick has the following commands: say hi, me waves."]);
        assert_eq!(t.cmd(alice, "/ptlist"), ["Item stick has the following commands: say hi, me waves."]);
        assert!(on_left_click(&mut t.ess, &mut t.host, alice));
        t.tick(1);
        let ran: Vec<String> = t.host.player_commands.iter().map(|(_, c, _)| c.clone()).collect();
        assert_eq!(ran, ["say hi", "me waves"]);
        assert_eq!(t.cmd(alice, "/pt r:say hi"), ["Command say hi removed from stick."]);
        assert_eq!(t.cmd(alice, "/ptt"), ["All of your power tools have been disabled."]);
        assert!(!on_left_click(&mut t.ess, &mut t.host, alice));
        assert_eq!(t.cmd(alice, "/ptt"), ["All of your power tools have been enabled."]);
        assert_eq!(t.cmd(alice, "/pt kill {player}"), ["kill {player} command assigned to stick."]);
        // {player} commands fire on hitting a player, not on a click.
        assert!(!on_left_click(&mut t.ess, &mut t.host, alice));
        let bob = t.join("Bob");
        assert!(on_attack_player(&mut t.ess, &mut t.host, alice, bob));
        t.tick(1);
        assert_eq!(t.host.player_commands.last().unwrap().1, "kill Bob");
        assert_eq!(t.cmd(alice, "/pt"), ["All commands removed from stick."]);
        assert_eq!(t.cmd(alice, "/pt d:"), ["All powertool commands have been cleared."]);
        assert_eq!(t.cmd(alice, "/ptlist"), ["You have no power tools assigned."]);
        assert_eq!(t.console("/pt Bob stick say hello"), ["say hello command assigned to stick."]);
    }

    #[test]
    fn every_command_is_registered() {
        let t = TestServer::new();
        for name in ["anvil", "book", "cartographytable", "clearinventory", "clearinventoryconfirmtoggle", "condense", "createkit", "delkit", "disposal", "enchant", "enderchest", "exp", "firework", "give", "grindstone", "hat", "item", "itemdb", "itemlore", "itemname", "kit", "kitreset", "loom", "more", "potion", "powertool", "powertoollist", "powertooltoggle", "recipe", "repair", "showkit", "skull", "smithingtable", "stonecutter", "unlimited", "workbench"] {
            assert!(t.ess.commands.contains_key(name), "{name}");
        }
    }
}
