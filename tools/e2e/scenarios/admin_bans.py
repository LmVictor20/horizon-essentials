# Admin (L3): kick, ban, tempban and unban a second player; banned players are refused when they rejoin.
import json
import time


def op(t, bot):
    t.console(f"op {bot.name}")
    t.wait_log(rf"Made {bot.name} a server operator|Opped {bot.name}")


def disconnected(bot, timeout=10):
    return bot.expect_event(lambda e: e.get("ev") == "disconnect", timeout, "a disconnect")["reason"]


def refused(t, name):
    """Joining fails; returns why."""
    try:
        bot = t.bot(name)
    except AssertionError as e:
        return str(e)
    bot.quit()
    raise AssertionError(f"{name} could join")


def test_kick_with_reason(t):
    alice = t.bot("Alice")
    op(t, alice)
    bob = t.bot("Bob")
    alice.cmd("/kick Bob Take a break")
    alice.expect(r"^Player Alice kicked Bob for Take a break\.$")
    assert "Take a break" in disconnected(bob)
    t.wait_log(r"Player Alice kicked Bob for Take a break")


def test_ban_refuses_rejoin_until_unban(t):
    alice = t.bot("Alice")
    op(t, alice)
    bob = t.bot("Bob")
    alice.cmd("/ban Bob Griefing")
    alice.expect(r"^Player Alice banned Bob for: Griefing\.$")
    assert "Griefing" in disconnected(bob)
    banned = json.loads((t.server_dir / "banned-players.json").read_text())
    assert any(b["name"] == "Bob" and b["reason"] == "Griefing" and b["expires"] == "forever" for b in banned), banned
    why = refused(t, "Bob")
    assert "banned" in why and "Griefing" in why, why
    alice.cmd("/unban Bob")
    alice.expect(r"^Player Alice unbanned Bob$")
    t.bot("Bob").cmd("/list")


def test_tempban_refuses_rejoin(t):
    alice = t.bot("Alice")
    op(t, alice)
    bob = t.bot("Bob")
    alice.cmd("/tempban Bob 1h Spamming")
    alice.expect(r"^Player Alice temporarily banned Bob for (59 minutes 5\d seconds|1 hour): Spamming\.$")
    assert "temporarily banned" in disconnected(bob)
    banned = json.loads((t.server_dir / "banned-players.json").read_text())
    assert any(b["name"] == "Bob" and b["expires"] != "forever" for b in banned), banned
    why = refused(t, "Bob")
    assert "banned" in why and "Spamming" in why, why


def test_banip_and_unbanip(t):
    # Every bot connects from 127.0.0.1, so the ban targets another address.
    alice = t.bot("Alice")
    op(t, alice)
    t.console("banip 10.9.9.9 Proxy")
    alice.expect(r"^Player Console banned IP address 10\.9\.9\.9 for: Proxy\.$")
    t.wait_log(r"Player Console banned IP address 10\.9\.9\.9 for: Proxy")
    banned = json.loads((t.server_dir / "banned-ips.json").read_text())
    assert any(b["ip"] == "10.9.9.9" and b["reason"] == "Proxy" for b in banned), banned
    alice.cmd("/unbanip 10.9.9.9")
    alice.expect(r"^Player Alice unbanned IP: 10\.9\.9\.9$")
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if not json.loads((t.server_dir / "banned-ips.json").read_text()):
            return
        time.sleep(0.2)
    raise AssertionError("10.9.9.9 is still in banned-ips.json")
