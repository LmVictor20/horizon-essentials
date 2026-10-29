// horizon-essentials area `items` (pure logic, standalone-tested): Items & inventory: item, give, more, unlimited, itemdb, itemname, itemlore, enchant, repair, potion, firework, skull, book, condense, clearinventory*, enderchest, disposal, hat, kits, powertool*, exp, recipe, workstations.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    let _ = ess;
}

// API stubs (tests first): implemented in the next commit.
pub fn item_stack(_ess: &mut Ess, _host: &mut dyn Host, _name: &str) -> Result<Item, CmdError> {
    Err(CmdError::Quiet)
}
pub fn component_plain(_c: &serde_json::Value) -> String {
    String::new()
}
pub fn give_kit(_ess: &mut Ess, _host: &mut dyn Host, _player: Uuid, _kit: &str) -> Result<bool, CmdError> {
    Err(CmdError::Quiet)
}
pub fn on_use_item(_ess: &mut Ess, _host: &mut dyn Host, _player: Uuid, _on_block: bool) {}
pub fn on_left_click(_ess: &mut Ess, _host: &mut dyn Host, _player: Uuid) -> bool {
    false
}
pub fn on_attack_player(_ess: &mut Ess, _host: &mut dyn Host, _attacker: Uuid, _defender: Uuid) -> bool {
    false
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
        // custom_items.yml aliases resolve too.
        assert_eq!(t.cmd(alice, "/itemdb snad")[0], "Item: SAND");
        // Potions and spawners keep their data.
        let stack = item_stack(&mut t.ess, &mut t.host, "strongswiftnesspotion").unwrap();
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
        assert_eq!(t.cmd(alice, "/more 60"), ["Your stack has been set to its default size, 64."]);
        assert_eq!(t.cmd(alice, "/more"), ["Error: You already have a full stack."]);
        hold(&mut t, alice, it("ender_pearl", 1));
        t.cmd(alice, "/more");
        assert_eq!(held(&mut t, alice).unwrap().count, 16);
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
        assert_eq!(rockets[1].count, 2);
        assert_eq!(rockets[1].components["fireworks"]["explosions"].as_array().unwrap().len(), 3);
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
        assert_eq!(t.cmd(bob, "/ci Alice"), ["Error: Player not found."]);
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
        assert!(!usage.last().unwrap().contains("protection,"));
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
        assert_eq!(t.console("/recipe torch"), ["Recipe for torch (1 of 1)", "1X | 2X | 2X", "3X | 2X | 2X", "2X | 2X | 2X", "Where: 1X is coal 2X is nothing 3X is stick "]);
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
