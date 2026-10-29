// horizon-essentials area `social` (pure logic, standalone-tested): Chat & social: msg, r, mail, me, ignore, socialspy, helpop, broadcast*, the Chat module (formats, local/shout/question), afk, nick, realname, motd, rules, info, customtext, help, join/quit messages, mute.
// Owned by one team member; register commands and listeners in `register`.

use crate::game::ess_core::*;

pub fn register(ess: &mut Ess) {
    let _ = ess;
}

/// User.notifyOfMail: called by the core before each command of a player with essentials.mail.
pub fn notify_mail(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    let _ = (ess, host, player);
}
