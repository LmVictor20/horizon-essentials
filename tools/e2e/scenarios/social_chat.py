# Area social: chat as vanilla, nicknames in chat, AFK, broadcast and the MOTD on join.


def test_motd_on_join(t):
    alice = t.bot("Alice")
    alice.expect(r"^Welcome, Alice!$")
    alice.expect(r"^Type /help for a list of commands\.$")


def test_plain_chat_stays_signed_chat(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    alice.chat("hello bob")
    event = bob.expect_event(lambda e: e.get("ev") == "chat" and "hello bob" in e.get("text", ""), 10, "Alice's chat")
    assert event["sender"] == alice.uuid.replace("-", ""), event


def test_nick_in_chat(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    t.console("op Alice")
    t.wait_log(r"Made Alice a server operator|Opped Alice")
    alice.cmd("/nick Ali")
    alice.expect(r"^Your nickname is now ~Ali\.$")
    alice.chat("hi there")
    bob.expect(r"^<~Ali> hi there$")
    bob.cmd("/realname Ali")
    bob.expect(r"^~Ali is Alice$")


def test_afk_and_broadcast(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    alice.cmd("/afk")
    alice.expect(r"^You are now AFK\.$")
    bob.expect(r"^\* Alice is now AFK\.$")
    bob.cmd("/msg Alice are you there")
    bob.expect(r"^Alice is currently AFK and may not respond\.$")
    alice.cmd("/afk")
    alice.expect(r"^You are no longer AFK\.$")
    t.console("broadcast Server restart soon")
    alice.expect(r"^\[Broadcast\] Server restart soon$")
    bob.expect(r"^\[Broadcast\] Server restart soon$")


def test_me_and_helpop(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    t.console("op Bob")
    t.wait_log(r"Made Bob a server operator|Opped Bob")
    alice.cmd("/me waves")
    bob.expect(r"^\* Alice waves$")
    alice.cmd("/helpop I am stuck")
    bob.expect(r"^\[HelpOp\] Alice: I am stuck$")
