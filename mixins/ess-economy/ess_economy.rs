// horizon-essentials area `economy` (pure logic, standalone-tested): Economy & signs: balance, balancetop, pay*, eco, sell, worth, setworth, command costs, every Essentials sign.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    let _ = ess;
}

/// An exact decimal (BigDecimal).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dec {
    m: i128,
    s: u32,
}
impl Dec {
    pub fn parse(_s: &str) -> Option<Dec> {
        todo!()
    }
    pub fn add(&self, _o: &Dec) -> Dec {
        todo!()
    }
    pub fn sub(&self, _o: &Dec) -> Dec {
        todo!()
    }
    pub fn mul(&self, _o: &Dec) -> Dec {
        todo!()
    }
    pub fn div_exact(&self, _o: &Dec) -> Option<Dec> {
        todo!()
    }
    pub fn cmp(&self, _o: &Dec) -> std::cmp::Ordering {
        todo!()
    }
}
impl std::fmt::Display for Dec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}e-{}", self.m, self.s)
    }
}
pub fn parse_amount(_s: &str) -> Option<Dec> {
    todo!()
}
pub fn display_currency(_ess: &Ess, _v: &Dec) -> String {
    todo!()
}
pub fn display_currency_exactly(_ess: &Ess, _v: &Dec) -> String {
    todo!()
}
pub fn short_currency(_ess: &Ess, _v: &Dec) -> String {
    todo!()
}
pub fn charge_command_cost(_ess: &mut Ess, _host: &mut dyn Host, _player: Uuid, _key: &str) -> Result<(), CmdError> {
    todo!()
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
        assert!(t.cmd(alice, "/bal Bob extra")[0].contains("balance"));
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
        t.console("eco set Alice -50");
        assert_eq!(money(&mut t, alice), "0");
        t.console("eco set Alice 990");
        assert_eq!(t.console("eco give Alice 20"), ["Error: This transaction would exceed the balance limit for this account."]);
        assert_eq!(money(&mut t, alice), "990");
    }

    #[test]
    fn min_money_is_always_negative_or_zero() {
        let mut t = TestServer::with_config("min-money: 100");
        let alice = t.join("Alice");
        t.console("eco set Alice -500");
        assert_eq!(money(&mut t, alice), "-100");
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
        let expected = format!("\",\"{}\",5,money,$,\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",5,", uuid_string(alice));
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

    fn shop() -> (TestServer, Uuid) {
        let mut t = TestServer::new();
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
        assert_eq!(money(&mut t, alice), "50");
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
        assert_eq!(money(&mut t, alice), "40");
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
        assert_eq!(lines[0], "Error: You are not allowed to sell these named items: Precious");
        assert_eq!(count(&mut t, alice, "diamond"), 1);
        let mut t = TestServer::with_config("allow-selling-named-items: true");
        let alice = t.join("Alice");
        t.op("Alice");
        t.cmd(alice, "/setworth diamond 10");
        t.host.p(alice).inventory[0] = Some(named);
        assert_eq!(t.cmd(alice, "/sell hand"), ["$10 has been added to your account.", "Sold for $10 (1 diamond at $10 each)."]);
    }

    #[test]
    fn sell_multipliers_and_stacks() {
        let mut t = TestServer::with_config("sell-multipliers:\n  default: 1.0\n  vip: 2.5\ntrade-in-stacks:\n  ironingot: true");
        let alice = t.join("Alice");
        t.op("Alice");
        t.cmd(alice, "/setworth diamond 10");
        t.cmd(alice, "/setworth iron_ingot 1");
        give(&mut t, alice, 0, "diamond", 1);
        assert_eq!(t.cmd(alice, "/sell hand")[1], "Sold for $10 (1 diamond at $10 each).");
        t.permissions("groups:\n  default:\n    default: true\nusers:\n  alice:\n    permissions: [essentials.sell.multiplier.vip]\n");
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
