# Core requests from area `social`

1. **`ess_testkit::kit::scheduler_tasks_run_on_ticks` assumes a join sends nothing.** Joins now send the MOTD
   (delay-motd 0) and "You have no new mail." like EssentialsX, so the test sees those lines. Please drain after
   the join (`let alice = t.join("Alice"); t.said(alice);`). Until then `cargo test -p ess-standalone` (all
   areas) has this one failure; `cargo test -p ess-standalone ess_social` is green.
2. **Command listeners only see Essentials' own commands.** `Ess::dispatch` runs `listeners.command` for labels
   Essentials claims, so `mute-commands` / `socialspy-commands` (and AFK activity on commands) miss vanilla and
   other mods' commands (`/say`, `/tellraw`...). A hook for every command line (e.g. `Ess::on_any_command` from
   the native_command hook, before `claims`) would cover them.
3. **The previous account name on join.** `on_join` overwrites `last-account-name` before the join listeners
   run, so `custom-new-username-message` / `{OLDUSERNAME}` / `reset-nick-on-name-change` can't detect a renamed
   account. Passing the old name to join listeners (or setting it after them) would allow it.
