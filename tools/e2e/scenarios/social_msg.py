# Area social: private messages, replies and ignore between two real players.


def test_msg_and_reply(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    alice.cmd("/msg Bob hello there")
    alice.expect(r"^\[me -> Bob\] hello there$")
    bob.expect(r"^\[Alice -> me\] hello there$")
    bob.cmd("/r hi back")
    bob.expect(r"^\[me -> Alice\] hi back$")
    alice.expect(r"^\[Bob -> me\] hi back$")


def test_ignore_blocks_msg_and_chat(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    t.console("op Bob")
    t.wait_log(r"Made Bob a server operator|Opped Bob")
    bob.cmd("/ignore Alice")
    bob.expect(r"^You ignore player Alice from now on\.$")
    alice.cmd("/msg Bob psst")
    alice.chat("hello everyone")
    # Alice still sees her own chat line; Bob gets neither.
    alice.expect(r"<Alice> hello everyone")
    bob.expect_none(r"psst|hello everyone", seconds=3)
    bob.cmd("/unignore Alice")
    bob.expect(r"^You are not ignoring player Alice anymore\.$")
    alice.chat("again")
    bob.expect(r"again")
