# Admin (L3): vanish hides a player from /list; /seen, /whois, /gc, /getpos answer on a real server.


def op(t, bot):
    t.console(f"op {bot.name}")
    t.wait_log(rf"Made {bot.name} a server operator|Opped {bot.name}")


def test_vanish_hides_from_list(t):
    alice = t.bot("Alice")
    op(t, alice)
    bob = t.bot("Bob")
    bob.cmd("/list")
    bob.expect(r"^There are 2 out of maximum \d+ players online\.$")
    alice.cmd("/vanish")
    alice.expect(r"^Vanish for Alice: enabled$")
    alice.expect(r"^You are now completely invisible")
    bob.cmd("/list")
    bob.expect(r"^There are 1 out of maximum \d+ players online\.$")
    bob.expect(r"^default: Bob$")
    alice.cmd("/list")
    alice.expect(r"^There are 1/1 out of maximum \d+ players online\.$")
    alice.expect(r"^default: \[HIDDEN\]Alice, Bob$")
    alice.cmd("/vanish off")
    alice.expect(r"^Vanish for Alice: disabled$")
    bob.cmd("/list")
    bob.expect(r"^default: Alice, Bob$")


def test_seen_whois_gc_getpos(t):
    alice = t.bot("Alice")
    op(t, alice)
    bob = t.bot("Bob")
    alice.cmd("/seen Bob")
    alice.expect(r"^Player Bob has been online since")
    alice.cmd("/whois Bob")
    alice.expect(r"^ ====== WhoIs: Bob ======$")
    alice.expect(r"^ - IP Address: /127\.0\.0\.1$")
    bob.quit()
    import time
    time.sleep(2)
    alice.cmd("/seen Bob")
    alice.expect(r"^Player Bob has been offline since")
    alice.expect(r"^ - IP Address: 127\.0\.0\.1$")
    alice.cmd("/gc")
    alice.expect(r"^Current TPS = \d")
    alice.expect(r"^World \"world\": \d+ chunks")
    alice.cmd("/getpos")
    alice.expect(r"^Current World: world$")
    t.console("seen Zed")
    t.wait_log(r"Player Zed was never on this server")
