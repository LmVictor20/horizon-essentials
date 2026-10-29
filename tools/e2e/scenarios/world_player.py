# World area (L3): player utilities as an opped bot on a real server: gamemode, fly, heal, feed, god, time,
# weather, speed. The bot sees chat; the server's own commands check the state where vanilla can.


def opped(t, name="Alice"):
    bot = t.bot(name)
    t.console(f"op {name}")
    t.wait_log(rf"Made {name} a server operator|Opped {name}")
    return bot


def test_gamemode(t):
    alice = opped(t)
    alice.cmd("/gmc")
    alice.expect(r"^Set game mode creative for Alice\.$")
    t.console("execute if entity @a[name=Alice,gamemode=creative]")
    t.wait_log(r"Test passed")
    alice.cmd("/gamemode survival")
    alice.expect(r"^Set game mode survival for Alice\.$")
    t.console("execute if entity @a[name=Alice,gamemode=survival]")
    t.wait_log(r"Test passed")


def test_fly_and_speed(t):
    alice = opped(t)
    alice.cmd("/fly")
    alice.expect(r"^Set fly mode enabled for Alice\.$")
    alice.cmd("/fly")
    alice.expect(r"^Set fly mode disabled for Alice\.$")
    alice.cmd("/speed walk 2")
    alice.expect(r"^Set walking speed to 2 for Alice\.$")
    alice.cmd("/flyspeed 3")
    alice.expect(r"^Set flying speed to 3 for Alice\.$")


def test_heal_feed(t):
    alice = opped(t)
    t.console("damage Alice 6")
    alice.cmd("/heal")
    alice.expect(r"^You have been healed\.$")
    alice.cmd("/feed")
    alice.expect(r"^Your appetite was sated\.$")
    bob = t.bot("Bob")
    alice.cmd("/heal Bob")
    alice.expect(r"^Healed Bob\.$")
    bob.expect(r"^You have been healed\.$")


def test_god_blocks_damage(t):
    # Standing in lava: god mode keeps Alice alive (the damage hook), and she burns once it is off.
    import time
    alice = opped(t)
    alice.cmd("/god")
    alice.expect(r"^God mode enabled\.$")
    time.sleep(1.5)  # the hook sees god mode from the next second's mirror
    t.console("execute at Alice run setblock ~ ~ ~ minecraft:lava")
    alice.expect_none(r"Alice tried to swim in lava", seconds=6)
    alice.cmd("/god")
    alice.expect(r"^God mode disabled\.$")
    t.wait_log(r"Alice tried to swim in lava", timeout=15)


def test_time_and_weather(t):
    alice = opped(t)
    alice.cmd("/time set day")
    alice.expect(r"^The time was set to 0 ticks in: world\.$")
    alice.cmd("/time")
    alice.expect(r"^06:0\d or 6:0\d AM or \d+ ticks$")
    alice.cmd("/night")
    alice.expect(r"^The time was set to 14000 ticks in: world\.$")
    alice.cmd("/weather storm")
    alice.expect(r"^You set the weather to storm in world\.$")
    alice.cmd("/sun")
    alice.expect(r"^You set the weather to sun in world\.$")
    alice.cmd("/ptime noon")
    alice.expect(r"^Player time is set to 12:00 or 12:00 PM or 6000 ticks for: Alice\.$")
    alice.cmd("/pweather storm")
    alice.expect(r"^Player weather is set to storm for: Alice\.$")
