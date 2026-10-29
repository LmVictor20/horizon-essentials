# The smallest end-to-end check: the mod is in the server and answers a player's command.


def test_ping(t):
    alice = t.bot("Alice")
    alice.cmd("/ping")
    alice.expect(r"^Pong!$")
    alice.cmd("/echo hello there")
    alice.expect(r"^hello there$")


def test_vanilla_commands_still_work(t):
    # A command the mod doesn't claim reaches vanilla's dispatcher as before.
    alice = t.bot("Alice")
    t.console("op Alice")
    t.wait_log(r"Made Alice a server operator|Opped Alice")
    alice.cmd("/time set day")
    alice.expect(r"commands\.time\.set|Set the time")
