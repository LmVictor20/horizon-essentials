# tp area, end to end: warps, spawn, random teleport. The flat world's ground is y -60.
# A teleport's position packet and its message may come in either order: the helpers search everything a bot
# saw since a mark instead of consuming events.
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


def place(t, bot, x, z):
    m = mark(bot)
    t.console(f"tp {bot.name} {x} -60 {z}")
    landed(bot, x + 0.5, -60, z + 0.5, since=m)


def op(t, name):
    t.console(f"op {name}")
    t.wait_log(rf"Made {name} a server operator|Opped {name}")


def test_setwarp_and_warp(t):
    alice = t.bot("Alice")
    op(t, "Alice")
    place(t, alice, 40, -12)
    a = mark(alice)
    alice.cmd("/setwarp Shop")
    seen(alice, r"^Warp Shop set\.$", a)
    assert (t.server_dir / "essentials" / "warps" / "shop.yml").exists()
    place(t, alice, 0, 0)
    a = mark(alice)
    alice.cmd("/warp shop")
    seen(alice, r"^Warping to shop\.$", a)
    landed(alice, 40.5, -60, -11.5, a)
    a = mark(alice)
    alice.cmd("/warps")
    seen(alice, r"^Warps: Shop$", a)
    alice.cmd("/warpinfo Shop")
    seen(alice, r"^Information for warp Shop:$", a)
    seen(alice, r"^ - Location: \(world, 40, -60, -12\)$", a)


def test_setspawn_and_spawn(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    op(t, "Alice")
    place(t, alice, -25, 25)
    a = mark(alice)
    alice.cmd("/setspawn")
    seen(alice, r"^Spawn location set for group default\.$", a)
    place(t, alice, 9, 9)
    a, b = mark(alice), mark(bob)
    alice.cmd("/spawn")
    seen(alice, r"^Teleporting\.\.\.$", a)
    landed(alice, -24.5, -60, 25.5, a)
    alice.cmd("/spawn Bob")
    seen(bob, r"^Alice teleported you to spawn\.$", b)
    landed(bob, -24.5, -60, 25.5, b)


def test_tpr_lands_on_the_ground(t):
    alice = t.bot("Alice")
    op(t, "Alice")
    place(t, alice, 0, 0)
    a = mark(alice)
    alice.cmd("/settpr world center")
    seen(alice, r"^Set random teleport center\.$", a)
    alice.cmd("/settpr world maxrange 20")
    seen(alice, r"^Set random teleport maxrange to 20\.$", a)
    a = mark(alice)
    alice.cmd("/tpr")
    seen(alice, r"^Teleporting to a random location\.\.\.$", a)
    seen(alice, r"^You have been teleported to a random location\.$", a)
    pos = [e for e in alice.lines[a:] if e.get("ev") == "pos"]
    assert pos, "no teleport"
    assert abs(pos[-1]["y"] - -60) < 0.01, pos
    assert max(abs(pos[-1]["x"]), abs(pos[-1]["z"])) <= 21, pos
