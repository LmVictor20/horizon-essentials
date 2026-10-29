# Core requests from the economy area

1. **A post-command hook with the result** (`listeners.command_done: fn(&mut Ess, &mut dyn Host, Sender, &str /*name*/, &CmdResult)`).
   EssentialsCommand charges `command-costs` *after* the command succeeds (`isAffordableFor`, run, `charge`).
   The `command` listener runs before dispatch (and before the permission check), so the economy area checks
   the permission itself and charges up front: a command that then fails with an error still costs money,
   and "$5 has been taken from your account." comes before the command's own output.

2. **Reaching the players during the network tick.** Horizon's crate is `#![forbid(unsafe_code)]`, and while a
   guest's packet is handled the hub is out of `game.lan` (borrowed as `host: &mut LanHost`), so `GameHost`
   can't see players, inventories or send messages from a packet hook. Today the economy glue works around
   it (see `ess_glue_economy.rs`): sign right clicks run their interact listeners on the next tick; container
   opening and block breaking use a "detached" host (player snapshot from the last tick, messages delivered
   next tick, no inventories). A core `GameHost` variant over `(&mut Game, &mut LanHost)` (or the hub held as
   `Option` in a way GameHost can borrow from either place) would let every area's listeners decide in place.

3. **A shared right-click / dig hook.** `lan_guest_packet` and `lan_sign_update` contain macros
   (`matches!`, `log::warn!`), which horizon-mixins refuses for HEAD hooks, so `S::UseItemOn` and
   `S::PlayerAction` can't be intercepted there. The economy area hooks `lan_open_sign_editor`,
   `lan_open_container`, `mark_block_entity_updated` and `break_block_as_player` instead. Right clicks on
   other blocks / in the air (powertools, etc.) still have no hook; a macro-free helper called at the top of
   `lan_guest_packet` upstream (or a pinned Horizon with one) would give every area the interact event.

4. **`block_place` has no feed.** Nothing calls `Ess::on_block_place` in the game yet ([Protection]'s
   "no chest next to someone else's protected chest" is L1 only).

5. **Host::block for the level in place** is the tp area's (`tp_block`); until it is implemented, the in-game
   checks that look at neighbouring blocks (signs held up by a block, [Protection]) see nothing. The economy
   detached host reads blocks itself.
