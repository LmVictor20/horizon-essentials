# Core requests from the admin area

Things the admin area needs from the core (`mixins/essentials/`). Each one has a workaround in place for now.

1. **Permissions: expand plugin.yml children of configured nodes.** `Perms::configured` only matches the node itself,
   its `a.b.*` parents and `*`. Granting a parent node such as `essentials.exempt` in permissions.yml therefore
   doesn't grant its plugin.yml children (`essentials.kick.exempt`, `essentials.ban.exempt`,
   `essentials.sudo.exempt`...), and neither do `essentials.seen.extra` or `essentials.kits.*`-style parents.
   Bukkit (and LuckPerms) resolve children. Proposal: when no configured node decides, look up each configured
   granted (or `-`negated) node in `ess_meta::PERMISSIONS` and follow its `children`.
   *Workaround:* grant the child nodes one by one.
2. **Permissions: user nodes should override group nodes.** At each level, `configured` checks every `-node` before
   every `node`, across user and group nodes together. So a group's `-essentials.fly` beats a user's own
   `essentials.fly`. Permission plugins let the more specific holder win (user > group > inherited group).
   Proposal: resolve in layers (user nodes, then the primary group, then inherited groups), and let the first
   layer that mentions the node decide.
3. **Command listener for every player command.** `listeners.command` only sees the lines Essentials claims.
   EssentialsX applies `command-cooldowns` (and social spy and mute-commands) to every command through
   PlayerCommandPreprocessEvent, vanilla ones included. Proposal: in `hook_command`, call the command listeners
   for unclaimed player lines too, cancelling (`*ran = true`) when one answers `Allow::No`.
   *Workaround:* cooldowns and jail restrictions apply to Essentials commands only.
4. **Order of the jail check.** `Ess::dispatch` runs the command listeners before the `essentials.<command>`
   permission check. EssentialsX checks the permission first, then jail, so a jailed player without the node
   sees "You do not have access" rather than "You do the crime". This is minor; it only matters if strict parity
   is wanted.
5. **Events the jail listener still lacks:** a teleport event (EssentialsX sends jailed players' teleports back to
   the jail), the attacker in `DamageFn` (jailed players can't hit players), and game-mode changes (jailed players
   can't change game mode).
