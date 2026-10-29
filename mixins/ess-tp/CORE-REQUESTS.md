# Core requests from area `tp`

## 1. Core test broken by the newbie announcement (one-line fix)

`ess_testkit::kit::scheduler_tasks_run_on_ticks` fails once `tp` is registered: EssentialsSpawn announces a
first join (`newbies.announce-format`, default `&dWelcome {DISPLAYNAME}&d to the server!`) two ticks after the
join, so Alice's inbox is not empty after `t.tick(2)`. That is EssentialsX behaviour.

Fix (ess_testkit.rs, in that test): drain the welcome before scheduling, e.g.

```rust
let alice = t.join("Alice");
t.tick(2); t.said(alice);   // first-join welcome (tp area: newbies.announce-format)
```

or start the test with `TestServer::with_config("newbies:\n  announce-format: ''")`.

## 2. Display names (nice to have)

Messages such as `teleportRequest`, `teleportToPlayer`, `teleportAtoB` use EssentialsX's `getDisplayName()`
(nickname + prefix). `tp` uses the account name today (`ess_tp::display`). A core `Ess::display_name(host, uuid)`
that the social area fills would let every area agree.

## 3. Ignore lists for tpa (social)

`/tpa`, `/tpahere` silently drop requests from players the target ignores (`isIgnoredPlayer`). A core
`ess_areas::is_ignored(ess, by, who) -> bool` delegating to social would let `tp` honour it (TODO in ess_tp.rs).

## 4. Newbie kit (items)

`newbies.kit` is given on the first join (EssentialsSpawn calls `Kit.expandItems`). A core
`ess_areas::give_kit(ess, host, player, kit)` delegating to items would let `tp` do it (TODO in ess_tp.rs).
