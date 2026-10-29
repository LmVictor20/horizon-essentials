# Area social: mail sent to an offline player, notified and read after they join again.


def test_mail_read_after_rejoin(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    bob.quit()
    t.wait_log(r"Bob (lost connection|left the game)")
    alice.cmd("/mail send Bob Hello there")
    alice.expect(r"^Bob has been sent the following mail:$")
    alice.expect(r"^Hello there$")
    bob = t.bot("Bob")
    bob.expect(r"^You have 1 messages! Type /mail read to view your mail\.$")
    bob.cmd("/mail read")
    bob.expect(r"Mail: read -- Page 1/1")
    bob.expect(r"\[Alice\] Hello there$")
    bob.expect(r"^To clear your mail, type /mail clear\.$")
    bob.cmd("/mail clear")
    bob.expect(r"^Mail cleared!$")
