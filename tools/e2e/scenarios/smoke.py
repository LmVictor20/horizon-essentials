# The smallest end-to-end check: the mod is in the server and answers a player's command.


def test_ping(t):
    alice = t.bot("Alice")
    alice.cmd("/ping")
    alice.expect(r"^Pong!$")
    alice.cmd("/echo hello there")
    alice.expect(r"^hello there$")


def test_join_writes_userdata(t):
    import time
    alice = t.bot("Alice")
    alice.cmd("/ping")
    alice.expect(r"^Pong!$")
    users = t.server_dir / "essentials" / "userdata"
    alice.quit()
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        files = list(users.glob("*.yml"))
        if files and "last-account-name: Alice" in files[0].read_text():
            return
        time.sleep(0.2)
    raise AssertionError(f"no userdata for Alice in {users}: {list(users.iterdir()) if users.exists() else 'missing'}")


def test_console_runs_essentials_commands(t):
    t.console("ping")
    t.wait_log(r"Pong!")


def test_vanilla_commands_still_work(t):
    # A command the mod doesn't claim reaches vanilla's dispatcher as before.
    alice = t.bot("Alice")
    t.console("op Alice")
    t.wait_log(r"Made Alice a server operator|Opped Alice")
    # /seed: vanilla's own (Essentials overrides /time, /give, /tp... on purpose).
    alice.cmd("/seed")
    alice.expect(r"commands\.seed\.success|Seed")
