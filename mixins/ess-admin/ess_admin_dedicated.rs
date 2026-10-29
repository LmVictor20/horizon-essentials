// horizon-essentials glue for area `admin`, placed in game/dedicated.rs (game only):
// `crate::game::dedicated::ess_admin_dedicated`. It sees the dedicated server's private items: the
// operator/whitelist/ban `Lists` (kept from the sync_ops hook) and USER_CACHE. (Lists::can_login, the login
// refusal, holds `vec!` macros horizon-mixins can't hook: banned players get vanilla's refusal message.)

use horizon_client::server::user_lists::{BanEntry, NameAndId};
use std::sync::{Arc, Mutex};

/// The server's lists, shared with the login threads (set by the sync_ops hook at startup).
static LISTS: Mutex<Option<Arc<Mutex<super::Lists>>>> = Mutex::new(None);

/// HEAD of `Game::sync_ops`: remember the lists.
pub(crate) fn remember(lists: &Arc<Mutex<super::Lists>>) {
    *LISTS.lock().unwrap() = Some(lists.clone());
}

fn with_lists<T>(f: impl FnOnce(&mut super::Lists) -> T) -> Option<T> {
    let shared = LISTS.lock().unwrap().clone()?;
    let mut lists = shared.lock().unwrap();
    Some(f(&mut lists))
}

/// A player known to the user cache (no profile lookup over the network; offline names in offline mode).
pub(crate) fn lookup(name: &str) -> Option<(u128, String)> {
    let mut cache = super::USER_CACHE.lock().unwrap();
    let found = cache.as_mut()?.get_by_name(name, |_| None)?;
    Some((found.id, found.name))
}

fn profile(name: &str, online: Option<u128>) -> NameAndId {
    if let Some(id) = online {
        return NameAndId { id, name: name.to_string() };
    }
    lookup(name).map(|(id, name)| NameAndId { id, name }).unwrap_or_else(|| NameAndId::offline(name))
}

/// banned-players.json or banned-ips.json, as Essentials' `Ban`s.
pub(crate) fn bans(ip: bool) -> Vec<crate::game::ess_core::Ban> {
    use crate::game::ess_core::Ban;
    with_lists(|l| {
        if ip {
            l.ip_bans.entries().iter().map(|e| Ban { target: e.user.clone(), reason: e.reason.clone().unwrap_or_default(), source: e.source.clone(), expires: e.expires }).collect()
        } else {
            l.bans.entries().iter().map(|e| Ban { target: e.user.name.clone(), reason: e.reason.clone().unwrap_or_default(), source: e.source.clone(), expires: e.expires }).collect()
        }
    })
    .unwrap_or_default()
}

/// Adds (or replaces) a ban; `online` is the uuid of the player when known.
pub(crate) fn ban(ip: bool, ban: &crate::game::ess_core::Ban, online: Option<u128>) {
    let reason = Some(ban.reason.clone()).filter(|r| !r.is_empty());
    let user = (!ip).then(|| profile(&ban.target, online));
    with_lists(|l| {
        if ip {
            l.ip_bans.remove(&ban.target);
            l.ip_bans.add(BanEntry::new(ban.target.clone(), Some(ban.source.clone()), ban.expires, reason));
        } else if let Some(user) = user {
            l.bans.remove(&user);
            l.bans.add(BanEntry::new(user, Some(ban.source.clone()), ban.expires, reason));
        }
    });
}

pub(crate) fn unban(ip: bool, target: &str) -> bool {
    with_lists(|l| {
        if ip {
            l.ip_bans.remove(&target.to_string())
        } else {
            let user = l.bans.entries().iter().find(|e| e.user.name.eq_ignore_ascii_case(target)).map(|e| e.user.clone());
            user.is_some_and(|u| l.bans.remove(&u))
        }
    })
    .unwrap_or(false)
}

pub(crate) fn whitelisted(id: u128, name: &str) -> bool {
    with_lists(|l| l.white.contains(&NameAndId { id, name: name.to_string() })).unwrap_or(false)
}
