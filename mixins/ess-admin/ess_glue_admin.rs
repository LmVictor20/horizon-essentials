// horizon-essentials glue for area `admin` (game only): `crate::game::ess_glue_admin`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here). The ban
// lists, user cache and login check live in `ess_admin_dedicated` (a module inside game/dedicated.rs).

use super::dedicated::ess_admin_dedicated as dedicated;
use super::ess_glue::GameHost;
use crate::game::ess_core::*;
use horizon_protocol::packets::play::{self, Clientbound as C};

/// PlayerInfoUpdate's UPDATE_LISTED action.
const UPDATE_LISTED: u8 = 0x08;

impl GameHost<'_, '_> {
    pub(super) fn admin_lookup(&mut self, name: &str) -> Option<(Uuid, String)> {
        if self.game.options.dedicated.is_none() {
            return None;
        }
        dedicated::lookup(name)
    }

    /// Vanish: the player leaves the tab list of everyone but `seers` (entities stay tracked: Horizon's
    /// entity tracking is shared by all viewers, so the body is hidden only by the invisibility effect).
    pub(super) fn admin_set_hidden(&mut self, player: Uuid, hidden: bool, seers: &[Uuid]) {
        let Some(hub) = self.hub() else { return };
        let entry = play::PlayerInfo { uuid: player, listed: Some(!hidden), ..Default::default() };
        let packet = C::PlayerInfoUpdate(play::PlayerInfoUpdate { actions: UPDATE_LISTED, entries: vec![entry] });
        for g in hub.guests.iter().filter(|g| g.placed && g.uuid != player && !seers.contains(&g.uuid)) {
            g.send(&packet);
        }
    }

    pub(super) fn admin_bans(&mut self, ip: bool) -> Vec<Ban> {
        dedicated::bans(ip)
    }

    pub(super) fn admin_ban(&mut self, ip: bool, ban: &Ban) {
        let online = self.hub().and_then(|h| h.guests.iter().find(|g| g.name.eq_ignore_ascii_case(&ban.target)).map(|g| g.uuid));
        dedicated::ban(ip, ban, online);
    }

    pub(super) fn admin_unban(&mut self, ip: bool, target: &str) -> bool {
        dedicated::unban(ip, target)
    }

    /// TPS and MSPT from the server clock's recent tick times; memory from /proc.
    pub(super) fn admin_stats(&mut self) -> ServerStats {
        let mspt = self.game.clock.average_nanos() as f64 / 1_000_000.0;
        let tps = if mspt <= 50.0 { 20.0 } else { 1000.0 / mspt };
        let kib = |file: &str, key: &str| -> u64 {
            std::fs::read_to_string(file)
                .ok()
                .and_then(|t| t.lines().find(|l| l.starts_with(key)).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse().ok()))
                .unwrap_or(0)
        };
        let mut worlds = Vec::new();
        let here = self.game.dim.id().to_string();
        let tiles: usize = self.game.world.chunks.values().map(|c| c.block_entities.len()).sum();
        worlds.push((here, self.game.world.chunks.len() as u32, self.game.mobs.len() as u32, tiles as u32));
        for dim in self.game.levels.parked_dims() {
            let entities = self.game.levels.parked_mut(dim).map_or(0, |p| p.mobs.len());
            let (chunks, tiles) = self
                .game
                .levels
                .parked_chunks()
                .find(|(d, _)| *d == dim)
                .map_or((0, 0), |(_, c)| (c.len(), c.values().map(|c| c.block_entities.len()).sum::<usize>()));
            worlds.push((dim.id().to_string(), chunks as u32, entities as u32, tiles as u32));
        }
        ServerStats {
            tps,
            mspt,
            uptime_ms: self.game.started.elapsed().as_millis() as i64,
            memory_used_mb: kib("/proc/self/status", "VmRSS:") / 1024,
            memory_max_mb: kib("/proc/meminfo", "MemTotal:") / 1024,
            worlds,
        }
    }

    pub(super) fn admin_max_players(&mut self) -> usize {
        self.hub().map_or(20, |h| h.lan.shared.max_players())
    }

    /// `minecraft:custom/minecraft:play_time`: the online player's live counters, else their stats file.
    pub(super) fn admin_statistic(&mut self, player: Uuid, stat: &str) -> i64 {
        let Some((ty, key)) = stat.split_once('/') else { return 0 };
        let ty = ty.strip_prefix("minecraft:").unwrap_or(ty);
        if let Some(conn) = self.conn(player) {
            if let Some(play) = self.game.lan.as_ref().and_then(|h| h.play.get(&conn)) {
                return i64::from(play.stats.get(ty, key));
            }
        }
        let path = super::stats::player_json(&self.game.world_root, "stats", horizon_client::lan::playerdata::uuid_ints(player));
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["stats"][format!("minecraft:{ty}")][key].as_i64())
            .unwrap_or(0)
    }

    pub(super) fn admin_whitelisted(&mut self, player: Uuid) -> bool {
        let name = self.hub().and_then(|h| h.guests.iter().find(|g| g.uuid == player).map(|g| g.name.clone())).unwrap_or_default();
        dedicated::whitelisted(player, &name)
    }

    /// Horizon keeps no first-join time.
    pub(super) fn admin_first_played(&mut self, _player: Uuid) -> Option<i64> {
        None
    }

    /// Horizon's guests keep no Bukkit speeds: the vanilla defaults.
    pub(super) fn admin_speeds(&mut self, _player: Uuid) -> (f32, f32) {
        (0.1, 0.2)
    }
}
