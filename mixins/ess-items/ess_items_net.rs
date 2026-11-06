// horizon-essentials (area items), in the horizon_client library: `horizon_client::lan::hub::ess_items_net`.
// Horizon keeps the components it doesn't model (`StackData::other`) but sends players only some of them. The
// item meta Essentials writes (/itemlore, /skull, kits' `unbreakable` and `custom-model-data`) needs these on
// the client too: Hub::stack's HEAD hook (inventories and menus) builds the stack as usual, then adds them.

use horizon_protocol::buf::Writer;
use horizon_protocol::item::{Component, Stack, component};
use horizon_world::nbt::Tag;
use std::cell::Cell;
use std::ops::ControlFlow;

thread_local! {
    static INSIDE: Cell<bool> = const { Cell::new(false) };
}

fn string(t: &Tag) -> Option<String> {
    match t {
        Tag::String(s) => Some(s.to_string_lossy()),
        _ => None,
    }
}

fn int(t: &Tag) -> Option<i64> {
    match t {
        Tag::Byte(v) => Some(i64::from(*v)),
        Tag::Short(v) => Some(i64::from(*v)),
        Tag::Int(v) => Some(i64::from(*v)),
        Tag::Long(v) => Some(*v),
        _ => None,
    }
}

fn float(t: &Tag) -> Option<f32> {
    match t {
        Tag::Float(v) => Some(f32::from_bits(*v)),
        Tag::Double(v) => Some(f64::from_bits(*v) as f32),
        other => int(other).map(|i| i as f32),
    }
}

fn list(t: Option<&Tag>) -> Vec<Tag> {
    match t {
        Some(Tag::List(l)) => l.clone(),
        Some(Tag::IntArray(a)) => a.iter().map(|v| Tag::Int(*v)).collect(),
        _ => Vec::new(),
    }
}

/// `ResolvableProfile`: a name, or {name, id, properties}. Encoded as Component::Raw bytes.
fn profile_bytes(t: &Tag) -> Option<Vec<u8>> {
    let (name, id, properties) = if let Some(name) = string(t) {
        (Some(name), None, Vec::new())
    } else {
        let name = t.get("name").and_then(string);
        let id = match t.get("id") {
            Some(Tag::IntArray(a)) if a.len() == 4 => Some(a.iter().fold(0u128, |acc, v| (acc << 32) | u128::from(*v as u32))),
            _ => None,
        };
        let properties: Vec<(String, String, Option<String>)> = list(t.get("properties"))
            .iter()
            .filter_map(|p| Some((p.get("name").and_then(string)?, p.get("value").and_then(string)?, p.get("signature").and_then(string))))
            .collect();
        (name, id, properties)
    };
    if name.is_none() && id.is_none() && properties.is_empty() {
        return None;
    }
    let mut w = Writer::new();
    w.u8(0);
    w.bool(name.is_some());
    if let Some(name) = &name {
        w.string(name);
    }
    w.bool(id.is_some());
    if let Some(id) = id {
        w.uuid(id);
    }
    w.varint(properties.len() as i32);
    for (name, value, signature) in &properties {
        w.string(name);
        w.string(value);
        w.bool(signature.is_some());
        if let Some(signature) = signature {
            w.string(signature);
        }
    }
    for _ in 0..4 {
        w.bool(false);
    }
    Some(w.data)
}

/// The stack as Hub::stack makes it, plus lore, profile, unbreakable and custom_model_data.
pub(super) fn stack(hub: &mut super::Hub, s: &Option<crate::items::ItemStack>) -> ControlFlow<Option<Stack>> {
    if INSIDE.with(Cell::get) {
        return ControlFlow::Continue(());
    }
    let Some(data) = s.as_ref().and_then(|s| s.data()) else { return ControlFlow::Continue(()) };
    let wanted = ["minecraft:lore", "minecraft:profile", "minecraft:unbreakable", "minecraft:custom_model_data"];
    if !wanted.iter().any(|k| data.other(k).is_some()) {
        return ControlFlow::Continue(());
    }
    INSIDE.with(|i| i.set(true));
    let base = hub.stack(s);
    INSIDE.with(|i| i.set(false));
    let Some(mut stack) = base else { return ControlFlow::Break(None) };
    if let Some(lore) = data.other("minecraft:lore") {
        stack.added.push((component::LORE, Component::Lore(list(Some(lore)))));
    }
    if let Some(bytes) = data.other("minecraft:profile").and_then(profile_bytes) {
        stack.added.push((component::PROFILE, Component::Raw(bytes)));
    }
    if data.other("minecraft:unbreakable").is_some() {
        stack.added.push((component::UNBREAKABLE, Component::Unit));
    }
    if let Some(c) = data.other("minecraft:custom_model_data") {
        stack.added.push((
            component::CUSTOM_MODEL_DATA,
            Component::CustomModelData {
                floats: list(c.get("floats")).iter().filter_map(float).collect(),
                flags: list(c.get("flags")).iter().filter_map(|f| int(f).map(|v| v != 0)).collect(),
                strings: list(c.get("strings")).iter().filter_map(string).collect(),
                colors: list(c.get("colors")).iter().filter_map(|v| int(v).map(|v| v as i32)).collect(),
            },
        ));
    }
    ControlFlow::Break(Some(stack))
}
