# Items area: items, kits and plugin containers reach a real client (ess-bot records the inventory packets:
# `slots` = ContainerSetContent, `slot` = ContainerSetSlot, `open` = OpenScreen).


def stacks(e):
    if e.get("ev") == "slots":
        return [s for s in e["items"] if s]
    if e.get("ev") == "slot" and e.get("item"):
        return [e["item"]]
    return []


def expect_stack(bot, count, what):
    return bot.expect_event(lambda e: any(s["count"] == count for s in stacks(e)), 10, what)


def op(t, bot):
    t.console(f"op {bot.name}")
    t.wait_log(rf"Made {bot.name} a server operator|Opped {bot.name}")


def test_item_reaches_the_inventory(t):
    alice = t.bot("Alice")
    op(t, alice)
    alice.cmd("/item stone 10")
    alice.expect(r"^Giving 10 of stone$")
    expect_stack(alice, 10, "a stack of 10 stone in the inventory")
    alice.cmd("/more")
    expect_stack(alice, 64, "the stack filled to 64 by /more")


def test_console_give(t):
    alice = t.bot("Alice")
    t.console("give Alice diamond 5")
    t.wait_log(r"Giving 5 of diamond to Alice")
    expect_stack(alice, 5, "5 diamonds from the console's /give")


def test_kit_tools(t):
    bob = t.bot("Bob")
    bob.cmd("/kit tools")
    bob.expect(r"^Received kit tools\.$")
    # The inventory updates arrive before the message: look at everything recorded.
    seen = {s["item"] for e in bob.lines for s in stacks(e)}
    assert len(seen) >= 4, f"the kit's four tools: {seen}"
    bob.cmd("/kit tools")
    bob.expect(r"can't use that kit again for another")


def test_disposal_and_workbench_open(t):
    alice = t.bot("Alice")
    op(t, alice)
    alice.cmd("/disposal")
    alice.expect(r"^Opening disposal menu\.\.\.$")
    opened = alice.expect_event(lambda e: e.get("ev") == "open", 10, "the disposal screen")
    assert opened["title"] == "Disposal", opened
    alice.cmd("/workbench")
    alice.expect_event(lambda e: e.get("ev") == "open", 10, "the crafting screen")
    alice.cmd("/anvil")
    alice.expect_event(lambda e: e.get("ev") == "open", 10, "the anvil screen")
    alice.cmd("/ec")
    alice.expect_event(lambda e: e.get("ev") == "open", 10, "the ender chest screen")
