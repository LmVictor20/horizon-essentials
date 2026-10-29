// horizon-essentials glue for area `world` (game only): `crate::game::ess_glue_world`.
// The Host methods this area owns, over Horizon's Game and hub (ess_glue delegates them here).
// Each starts as the safe default ("not supported"); replace bodies as the area needs them.

use super::ess_glue::GameHost;
use crate::game::ess_core::*;

#[allow(unused_variables)]
impl GameHost<'_, '_> {
    pub(super) fn world_game_mode(&mut self, player: Uuid) -> GameMode {
        GameMode::Survival
    }
    pub(super) fn world_set_game_mode(&mut self, player: Uuid, mode: GameMode) {
    }
    pub(super) fn world_health(&mut self, player: Uuid) -> (f64, f64) {
        (20.0, 20.0)
    }
    pub(super) fn world_set_health(&mut self, player: Uuid, health: f64) {
    }
    pub(super) fn world_food(&mut self, player: Uuid) -> (i32, f32) {
        (20, 5.0)
    }
    pub(super) fn world_set_food(&mut self, player: Uuid, food: i32, saturation: f32) {
    }
    pub(super) fn world_set_fire_ticks(&mut self, player: Uuid, ticks: i32) {
    }
    pub(super) fn world_fire_ticks(&mut self, player: Uuid) -> i32 {
        0
    }
    pub(super) fn world_set_freeze_ticks(&mut self, player: Uuid, ticks: i32) {
    }
    pub(super) fn world_exp(&mut self, player: Uuid) -> i64 {
        0
    }
    pub(super) fn world_set_exp(&mut self, player: Uuid, total: i64) {
    }
    pub(super) fn world_flying(&mut self, player: Uuid) -> bool {
        false
    }
    pub(super) fn world_set_allow_flight(&mut self, player: Uuid, allow: bool) {
    }
    pub(super) fn world_allow_flight(&mut self, player: Uuid) -> bool {
        false
    }
    pub(super) fn world_set_flying(&mut self, player: Uuid, flying: bool) {
    }
    pub(super) fn world_set_fly_speed(&mut self, player: Uuid, speed: f32) {
    }
    pub(super) fn world_set_walk_speed(&mut self, player: Uuid, speed: f32) {
    }
    pub(super) fn world_set_invulnerable(&mut self, player: Uuid, invulnerable: bool) {
    }
    pub(super) fn world_set_player_time(&mut self, player: Uuid, time: Option<(i64, bool)>) {
    }
    pub(super) fn world_set_player_weather(&mut self, player: Uuid, downfall: Option<bool>) {
    }
    pub(super) fn world_entity_types(&mut self) -> Vec<String> {
        Vec::new()
    }
    pub(super) fn world_world_time(&mut self, world: &str) -> i64 {
        0
    }
    pub(super) fn world_set_world_time(&mut self, world: &str, time: i64) {
    }
    pub(super) fn world_weather(&mut self, world: &str) -> (bool, bool) {
        (false, false)
    }
    pub(super) fn world_set_weather(&mut self, world: &str, rain: bool, thunder: bool, duration: Option<i32>) {
    }
    pub(super) fn world_spawn_entity(&mut self, kind: &str, at: &Loc, snbt: Option<&str>) -> bool {
        false
    }
    pub(super) fn world_strike_lightning(&mut self, at: &Loc, effect_only: bool) {
    }
    pub(super) fn world_set_statistic(&mut self, player: Uuid, stat: &str, value: i64) {
    }
    pub(super) fn world_remove_entities(&mut self, world: &str, center: Option<&Loc>, radius: Option<f64>, kinds: &[String]) -> usize {
        0
    }
}

// ----- Network (Discord REST, GeoIP download, XMPP) ------------------------------------------------
// Requests run on their own threads (ureq is blocking); answers wait in NET until the next poll, so the server
// thread never blocks on the network.

use std::sync::Mutex;
use std::sync::mpsc;

struct Net {
    http_done: Vec<(u64, u16, String)>,
    tcp_events: Vec<(u64, Vec<u8>)>,
    tcp_writers: std::collections::HashMap<u64, mpsc::Sender<Vec<u8>>>,
}

static NET: Mutex<Option<Net>> = Mutex::new(None);

fn net<T>(f: impl FnOnce(&mut Net) -> T) -> T {
    let mut guard = NET.lock().unwrap_or_else(|e| e.into_inner());
    let n = guard.get_or_insert_with(|| Net { http_done: Vec::new(), tcp_events: Vec::new(), tcp_writers: Default::default() });
    f(n)
}

impl GameHost<'_, '_> {
    pub(super) fn world_http_request(&mut self, id: u64, method: &str, url: &str, headers: &[(String, String)], body: Option<&str>) {
        let (method, url, headers, body) = (method.to_string(), url.to_string(), headers.to_vec(), body.map(str::to_string));
        std::thread::spawn(move || {
            let mut req = ureq::request(&method, &url).timeout(std::time::Duration::from_secs(20));
            for (k, v) in &headers {
                req = req.set(k, v);
            }
            let result = match &body {
                Some(b) => req.send_string(b),
                None => req.call(),
            };
            let answer = match result {
                Ok(resp) => {
                    let status = resp.status();
                    (status, resp.into_string().unwrap_or_default())
                }
                Err(ureq::Error::Status(code, resp)) => (code, resp.into_string().unwrap_or_default()),
                Err(e) => (0, e.to_string()),
            };
            net(|n| n.http_done.push((id, answer.0, answer.1)));
        });
    }
    pub(super) fn world_http_poll(&mut self) -> Vec<(u64, u16, String)> {
        net(|n| std::mem::take(&mut n.http_done))
    }
    pub(super) fn world_tcp_open(&mut self, id: u64, host: &str, port: u16, tls: bool) {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        net(|n| n.tcp_writers.insert(id, tx));
        let host = host.to_string();
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            // TLS streams are not reachable through the crates Horizon depends on (ureq keeps rustls private).
            let stream = if tls { None } else { std::net::TcpStream::connect((host.as_str(), port)).ok() };
            let Some(mut stream) = stream else {
                net(|n| n.tcp_events.push((id, Vec::new())));
                return;
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));
            let mut buf = [0u8; 8192];
            loop {
                loop {
                    match rx.try_recv() {
                        Ok(data) if data.is_empty() => {
                            net(|n| n.tcp_events.push((id, Vec::new())));
                            return;
                        }
                        Ok(data) => {
                            if stream.write_all(&data).is_err() {
                                net(|n| n.tcp_events.push((id, Vec::new())));
                                return;
                            }
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => return,
                    }
                }
                match stream.read(&mut buf) {
                    Ok(0) => {
                        net(|n| n.tcp_events.push((id, Vec::new())));
                        return;
                    }
                    Ok(k) => net(|n| n.tcp_events.push((id, buf[..k].to_vec()))),
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(_) => {
                        net(|n| n.tcp_events.push((id, Vec::new())));
                        return;
                    }
                }
            }
        });
    }
    pub(super) fn world_tcp_send(&mut self, id: u64, data: &[u8]) {
        if !data.is_empty() {
            net(|n| n.tcp_writers.get(&id).map(|w| w.send(data.to_vec())));
        }
    }
    pub(super) fn world_tcp_close(&mut self, id: u64) {
        net(|n| n.tcp_writers.remove(&id).map(|w| w.send(Vec::new())));
    }
    pub(super) fn world_tcp_poll(&mut self) -> Vec<(u64, Vec<u8>)> {
        net(|n| std::mem::take(&mut n.tcp_events))
    }
}
