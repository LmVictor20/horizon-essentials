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
4. **`tools/e2e/run.py <area>` doesn't select `<area>_*.py`.** `collect` matches a file stem or `stem.test`
   exactly, so `run.py social` finds nothing for `social_msg.py`, `social_mail.py`, `social_chat.py`. Matching
   `file.stem.startswith(s + "_")` too would make `run.py social` run the area's files. Meanwhile:
   `python3 tools/e2e/run.py social_msg social_mail social_chat`.
5. **No hookable display name in the hub.** `Hub::player_display_name` and `lan::hub::display_name` contain
   macros (`json!`, `format!`) and `Hub::player_info` closures, so neither HEAD nor RETURN can target them. The
   glue works around it (chat with a nickname or prefix is re-rendered by Essentials as system chat; tab names are
   re-sent with `PlayerInfoUpdate` 0x20 whenever the player list changes). A macro-free seam (e.g. a small
   `fn display_override(&self, uuid) -> Option<Tag>` called from both) would let nicknames show in vanilla-signed
   chat and the join/leave lines too.
