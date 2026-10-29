// horizon-essentials: the feature areas, each registering its commands and listeners. Calls the core makes
// into one area (mail notification on every command) go through here so ess_core never names an area.

use crate::game::ess_core::{Ess, Host, Uuid};

pub fn register_all(ess: &mut Ess) {
    crate::game::ess_tp::register(ess);
    crate::game::ess_social::register(ess);
    crate::game::ess_admin::register(ess);
    crate::game::ess_items::register(ess);
    crate::game::ess_economy::register(ess);
    crate::game::ess_world::register(ess);
}

/// User.notifyOfMail before each command (the social area implements mail).
pub fn notify_mail(ess: &mut Ess, host: &mut dyn Host, player: Uuid) {
    crate::game::ess_social::notify_mail(ess, host, player);
}
