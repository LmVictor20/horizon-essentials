# Area economy: balances through the console's /eco, /pay between two real players, /balance and /baltop.


def test_eco_give_and_balance(t):
    alice = t.bot("Alice")
    t.console("eco give Alice 100")
    t.wait_log(r"\$100 added to Alice account\. New balance: \$100")
    alice.expect(r"^\$100 has been added to your account\.$")
    alice.cmd("/balance")
    alice.expect(r"^Balance: \$100$")
    t.console("eco take Alice 0.5")
    alice.expect(r"^\$0\.50 has been taken from your account\.$")
    alice.cmd("/bal")
    alice.expect(r"^Balance: \$99\.50$")


def test_pay_between_players(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    t.console("eco set Alice 50")
    alice.expect(r"^Your balance was set to \$50\.$")
    alice.cmd("/pay Bob 20")
    alice.expect(r"^\$20 has been sent to Bob\.$")
    bob.expect(r"^\$20 has been received from Alice\.$")
    bob.cmd("/balance")
    bob.expect(r"^Balance: \$20$")
    alice.cmd("/pay Bob 1000")
    alice.expect(r"^Error: You do not have sufficient funds\.$")


def test_paytoggle_refuses_payments(t):
    alice = t.bot("Alice")
    bob = t.bot("Bob")
    t.console("op Bob")
    t.wait_log(r"Made Bob a server operator|Opped Bob")
    t.console("eco set Alice 10")
    bob.cmd("/paytoggle")
    bob.expect(r"^You are no longer accepting payments\.$")
    alice.cmd("/pay Bob 5")
    alice.expect(r"^Bob is not accepting payment\.$")


def test_balancetop(t):
    alice = t.bot("Alice")
    t.bot("Bob")
    t.console("eco set Alice 300")
    t.console("eco set Bob 40")
    alice.expect(r"^Your balance was set to \$300\.$")
    alice.cmd("/baltop")
    alice.expect(r"^Top balances \(")
    alice.expect(r"Balancetop -- Page 1/1")
    alice.expect(r"^Server Total: \$340$")
    alice.expect(r"^1\. Alice, \$300$")
    alice.expect(r"^2\. Bob, \$40$")
