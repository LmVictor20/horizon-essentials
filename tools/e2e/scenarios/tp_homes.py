# tp area, end to end: homes, /back (after a teleport and a death), /top. The flat world's ground is y -60.
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


def test_sethome_and_home(t):
    alice = t.bot("Alice")
    place(t, alice, 12, 34)
    a = mark(alice)
    alice.cmd("/sethome")
    seen(alice, r"^Home set to current location\.$", a)
    place(t, alice, -40, 8)
    a = mark(alice)
    alice.cmd("/home")
    seen(alice, r"^Teleporting to home\.$", a)
    landed(alice, 12.5, -60, 34.5, a)
    alice.quit()  # userdata is written on quit (and every 30 s)
    users = t.server_dir / "essentials" / "userdata"
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if any("homes:" in f.read_text() for f in users.glob("*.yml")):
            return
        time.sleep(0.2)
    raise AssertionError("no homes in Alice's userdata")


def test_named_homes_list_and_delhome(t):
    alice = t.bot("Alice")
    op(t, "Alice")
    place(t, alice, 5, 5)
    a = mark(alice)
    alice.cmd("/sethome base")
    seen(alice, r"^Home set to current location\.$", a)
    place(t, alice, 25, 5)
    a = mark(alice)
    alice.cmd("/sethome mine")
    seen(alice, r"^Home set to current location\.$", a)
    a = mark(alice)
    alice.cmd("/home")
    seen(alice, r"^Homes: base, mine, bed$", a)
    a = mark(alice)
    alice.cmd("/home base")
    landed(alice, 5.5, -60, 5.5, a)
    a = mark(alice)
    alice.cmd("/delhome mine")
    seen(alice, r"^Home mine has been removed\.$", a)


def test_back_after_teleport_and_death(t):
    alice = t.bot("Alice")
    op(t, "Alice")
    place(t, alice, 7, 7)
    place(t, alice, 70, 7)
    a = mark(alice)
    alice.cmd("/back")
    seen(alice, r"^Returning to previous location\.$", a)
    landed(alice, 7.5, -60, 7.5, a)
    # Death: the default group has essentials.back.ondeath (an operator's plugin.yml default is false).
    t.console("deop Alice")
    t.wait_log(r"Made Alice no longer a server operator|De-opped Alice")
    a = mark(alice)
    t.console("kill Alice")
    seen(alice, r"^Use the /back command to return to your death point\.$", a, timeout=15)


def test_top(t):
    alice = t.bot("Alice")
    op(t, "Alice")
    a = mark(alice)
    # Vanilla's tp (inside execute, which Essentials doesn't claim) puts her in the dirt.
    t.console("execute in minecraft:overworld run tp Alice 3.5 -63 3.5")
    landed(alice, 3.5, -63, 3.5, a)
    a = mark(alice)
    alice.cmd("/top")
    seen(alice, r"^Teleporting to top\.$", a)
    landed(alice, 3.5, -60, 3.5, a)
