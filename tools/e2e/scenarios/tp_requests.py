# tp area, end to end: teleport requests between two real players (default group: no op needed).
# The flat world's ground is y -60 (grass at -61). A teleport's position packet and its message may come in
# either order, so these helpers search everything a bot saw since a mark instead of consuming events.
import re
import time


def mark(bot):
    return len(bot.lines)


def seen(bot, pattern, since=0, timeout=10):
    rx = re.compile(pattern)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for e in bot.lines[since:]:
            if e.get("ev") in ("system", "chat") and rx.search(e.get("text", "")):
                return e["text"]
        time.sleep(0.1)
    texts = [e.get("text") for e in bot.lines[since:] if e.get("ev") in ("system", "chat")]
    raise AssertionError(f"{bot.name}: no line matching {pattern!r}; saw {texts}")


def landed(bot, x, y, z, since=0, within=0.6, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for e in bot.lines[since:]:
            if e.get("ev") == "pos" and all(abs(e[k] - v) <= within for k, v in (("x", x), ("y", y), ("z", z))):
                return e
        time.sleep(0.1)
    pos = [e for e in bot.lines[since:] if e.get("ev") == "pos"]
    raise AssertionError(f"{bot.name}: no teleport near {x} {y} {z}; positions {pos}")


def join(t, name):
    """A first join: wait for EssentialsSpawn's newbie teleport and welcome before moving the player."""
    bot = t.bot(name)
    seen(bot, rf"^Welcome {name} to the server!$")
    time.sleep(0.5)
    return bot


def place(t, bot, x, z):
    """The console moves a player (Essentials' /tp) and waits for their client to see it."""
    m = mark(bot)
    t.console(f"tp {bot.name} {x} -60 {z}")
    landed(bot, x + 0.5, -60, z + 0.5, since=m)


def test_tpa_and_tpaccept(t):
    alice = join(t, "Alice")
    bob = join(t, "Bob")
    place(t, bob, 20, 20)
    place(t, alice, -10, -10)
    a, b = mark(alice), mark(bob)
    alice.cmd("/tpa Bob")
    seen(alice, r"^Request sent to Bob\.$", a)
    seen(bob, r"^Alice has requested to teleport to you\.$", b)
    seen(bob, r"^To teleport, type /tpaccept\.$", b)
    bob.cmd("/tpaccept")
    seen(bob, r"^Teleport request accepted\.$", b)
    seen(alice, r"^Bob accepted your teleport request\.$", a)
    seen(alice, r"^Teleporting to Bob\.$", a)
    landed(alice, 20.5, -60, 20.5, a)


def test_tpahere_and_tpdeny(t):
    alice = join(t, "Alice")
    bob = join(t, "Bob")
    place(t, alice, 15, -5)
    place(t, bob, -15, 5)
    a, b = mark(alice), mark(bob)
    alice.cmd("/tpahere Bob")
    seen(bob, r"^Alice has requested that you teleport to them\.$", b)
    bob.cmd("/tpaccept")
    landed(bob, 15.5, -60, -4.5, b)
    seen(alice, r"^Teleporting\.\.\.$", a)

    a, b = mark(alice), mark(bob)
    alice.cmd("/tpa Bob")
    seen(bob, r"^Alice has requested to teleport to you\.$", b)
    bob.cmd("/tpdeny")
    seen(bob, r"^Teleport request denied\.$", b)
    seen(alice, r"^Bob denied your teleport request\.$", a)
    b = mark(bob)
    bob.cmd("/tpaccept")
    seen(bob, r"You do not have a pending request\.", b)


def test_tp_and_tphere_as_operator(t):
    alice = join(t, "Alice")
    bob = join(t, "Bob")
    t.console("op Alice")
    t.wait_log(r"Made Alice a server operator|Opped Alice")
    place(t, bob, 30, 30)
    a = mark(alice)
    alice.cmd("/tp Bob")
    seen(alice, r"^Teleporting to Bob\.$", a)
    landed(alice, 30.5, -60, 30.5, a)
    place(t, alice, -30, 30)
    b = mark(bob)
    alice.cmd("/tphere Bob")
    seen(bob, r"^Teleporting\.\.\.$", b)
    landed(bob, -29.5, -60, 30.5, b)
