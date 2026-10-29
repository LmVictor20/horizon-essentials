# Admin (L3): a jail is set, a second player is jailed (teleported, commands refused) and freed back.
import time


def op(t, bot):
    t.console(f"op {bot.name}")
    t.wait_log(rf"Made {bot.name} a server operator|Opped {bot.name}")


def last_pos(bot):
    return [e for e in bot.lines if e.get("ev") == "pos"][-1]


def test_jail_and_unjail(t):
    alice = t.bot("Alice")
    op(t, alice)
    cell = last_pos(alice)
    alice.cmd("/setjail cell")
    alice.expect(r"^Jail cell has been set\.$")
    alice.cmd("/jails")
    alice.expect(r"^Jails: cell$")
    bob = t.bot("Bob")
    start = last_pos(bob)
    # Bob starts somewhere else than the cell (random spawn spread); he is sent there and back.
    alice.cmd("/jail Bob cell")
    alice.expect(r"^Player Bob jailed\.$")
    alice.expect(r"^Player Bob jailed by Alice\.$")
    bob.expect_pos(cell["x"], cell["y"], cell["z"], within=1.0)
    bob.expect(r"^You have been jailed!$")
    time.sleep(0.5)
    bob.cmd("/seen Alice")
    bob.expect(r"^You do the crime, you do the time\.$")
    alice.cmd("/jailedplayers")
    alice.expect(r"^Jailed Players: Bob$")
    alice.cmd("/unjail Bob")
    bob.expect(r"^You have been released!$")
    alice.expect(r"^Player Bob unjailed\.$")
    bob.expect_pos(start["x"], start["y"], start["z"], within=1.5)
    bob.cmd("/seen Alice")
    bob.expect(r"^Player Alice has been online since")
