// ess-bot: one scripted Minecraft Java 26.3 player for horizon-essentials' end-to-end tests.
//
//   ess-bot --server 127.0.0.1:25565 --name Alice
//
// It logs in offline and goes through configuration as vanilla's client does (the flow is horizon-bots'
// `join`), answers keep-alives, pings, teleports and chunk batches, then:
// - reads JSON lines on stdin:
//     {"op":"cmd","line":"/home base"}   a command (the leading slash is optional)
//     {"op":"chat","text":"hello"}       an unsigned chat message
//     {"op":"move","x":1.5,"y":64,"z":2.5}   one position update (also ends a client tick)
//     {"op":"quit"}
// - prints one JSON line per event on stdout:
//     {"ev":"joined","name":..}  {"ev":"system","text":..,"overlay":bool}
//     {"ev":"chat","sender":"uuid","text":..}  {"ev":"disguised","text":..}
//     {"ev":"pos","x":..,"y":..,"z":..,"yaw":..,"pitch":..}  {"ev":"disconnect","reason":..}
// Text components are flattened to plain text (translations print their key and arguments).

use horizon_protocol::{
    PROTOCOL_VERSION,
    buf::{Reader, Writer},
    connection::{ConnReader, ConnWriter, Connection},
    crypto::offline_uuid,
    ids,
    packets::{
        Empty, configuration,
        handshake::{self, Intention},
        login::{self, ServerHello},
        play,
    },
};
use horizon_world::nbt::Tag;
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn raw(id: i32, write: impl FnOnce(&mut Writer)) -> Vec<u8> {
    let mut w = Writer::new();
    w.varint(id);
    write(&mut w);
    w.data
}

fn emit(event: Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{event}");
    let _ = out.flush();
}

/// A chat component (NBT) as plain text: `text`, then `extra`; a translation as `key[arg, ...]`.
fn plain(tag: &Tag) -> String {
    match tag {
        Tag::String(s) => s.to_string_lossy(),
        Tag::List(items) => items.iter().map(plain).collect(),
        Tag::Compound(_) => {
            let mut out = String::new();
            if let Some(Tag::String(s)) = tag.get("text") {
                out.push_str(&s.to_string_lossy());
            }
            if let Some(Tag::String(key)) = tag.get("translate") {
                out.push_str(&key.to_string_lossy());
                if let Some(Tag::List(args)) = tag.get("with") {
                    let args: Vec<String> = args.iter().map(plain).collect();
                    out.push_str(&format!("[{}]", args.join(", ")));
                }
            }
            if let Some(Tag::List(extra)) = tag.get("extra") {
                out.extend(extra.iter().map(plain));
            }
            out
        }
        _ => String::new(),
    }
}

/// Handshake, offline login and configuration (horizon-bots' `join`); the connection in the play state.
fn join(host: &str, port: u16, name: &str) -> Result<Connection, String> {
    let mut c = Connection::connect(host, port, Duration::from_secs(10)).map_err(err)?;
    c.send(
        &handshake::Serverbound::Intention(Intention {
            protocol: PROTOCOL_VERSION,
            host: host.to_string(),
            port,
            intent: handshake::LOGIN,
        })
        .encode(),
    )
    .map_err(err)?;
    c.send(&login::Serverbound::Hello(ServerHello { name: name.into(), uuid: offline_uuid(name) }).encode())
        .map_err(err)?;
    loop {
        let (id, body) = c.recv().map_err(err)?;
        match login::Clientbound::decode(id, &body).map_err(err)? {
            login::Clientbound::Compression(p) => c.set_compression(p.threshold),
            login::Clientbound::LoginFinished(_) => {
                c.send(&login::Serverbound::LoginAcknowledged(Empty).encode()).map_err(err)?;
                break;
            }
            login::Clientbound::Disconnect(p) => return Err(format!("refused: {}", p.reason_json)),
            login::Clientbound::Hello(_) => return Err("online-mode server; tests need online-mode=false".into()),
            login::Clientbound::CustomQuery(p) => c
                .send(
                    &login::Serverbound::CustomQueryAnswer(login::CustomQueryAnswer {
                        transaction: p.transaction,
                        payload: None,
                    })
                    .encode(),
                )
                .map_err(err)?,
            login::Clientbound::CookieRequest(p) => c
                .send(&login::Serverbound::CookieResponse(login::CookieResponse { key: p.key, payload: None }).encode())
                .map_err(err)?,
            login::Clientbound::Unknown(..) => {}
        }
    }
    use ids::configuration::clientbound as cb;
    c.send(&configuration::Serverbound::ClientInformation(configuration::ClientInformation::new(2)).encode())
        .map_err(err)?;
    loop {
        let (id, body) = c.recv().map_err(err)?;
        let reply = match id {
            cb::SELECT_KNOWN_PACKS
            | cb::KEEP_ALIVE
            | cb::PING
            | cb::CODE_OF_CONDUCT
            | cb::COOKIE_REQUEST
            | cb::DISCONNECT
            | cb::FINISH_CONFIGURATION => match configuration::Clientbound::decode(id, &body).map_err(err)? {
                configuration::Clientbound::SelectKnownPacks(p) => configuration::Serverbound::SelectKnownPacks(p),
                configuration::Clientbound::KeepAlive(p) => configuration::Serverbound::KeepAlive(p),
                configuration::Clientbound::Ping(p) => configuration::Serverbound::Pong(p),
                configuration::Clientbound::CodeOfConduct(_) => configuration::Serverbound::AcceptCodeOfConduct(Empty),
                configuration::Clientbound::CookieRequest(p) => {
                    configuration::Serverbound::CookieResponse(configuration::CookieResponse { key: p.key, payload: None })
                }
                configuration::Clientbound::Disconnect(_) => return Err("disconnected during configuration".into()),
                configuration::Clientbound::FinishConfiguration(_) => {
                    c.send(&configuration::Serverbound::FinishConfiguration(Empty).encode()).map_err(err)?;
                    return Ok(c);
                }
                _ => continue,
            },
            _ => continue,
        };
        c.send(&reply.encode()).map_err(err)?;
    }
}

/// The play state: answers what a client must answer and reports what the tests read.
fn read(mut reader: ConnReader, writer: Arc<Mutex<ConnWriter>>, position: Arc<Mutex<[f64; 3]>>) {
    use ids::play::clientbound as cb;
    use ids::play::serverbound as sb;
    let send = |body: Vec<u8>| lock(&writer).send(&body);
    loop {
        let (id, body) = match reader.recv() {
            Ok(packet) => packet,
            Err(e) => {
                emit(json!({"ev": "disconnect", "reason": e.to_string()}));
                std::process::exit(0);
            }
        };
        let mut r = Reader::new(&body);
        let sent = match id {
            cb::KEEP_ALIVE => r.i64().map_or(Ok(()), |v| send(raw(sb::KEEP_ALIVE, |w| w.i64(v)))),
            cb::PING => r.i32().map_or(Ok(()), |v| send(raw(sb::PONG, |w| w.i32(v)))),
            cb::CHUNK_BATCH_FINISHED => {
                send(play::Serverbound::ChunkBatchReceived(play::ChunkBatchReceived { chunks_per_tick: 64.0 }).encode())
            }
            cb::PLAYER_POSITION => match play::Clientbound::decode(id, &body) {
                Ok(play::Clientbound::PlayerPosition(p)) => {
                    let mut at = lock(&position);
                    // Relative flags: X 1, Y 2, Z 4.
                    let pos: [f64; 3] = std::array::from_fn(|i| {
                        if p.relatives & (1 << i) != 0 { at[i] + p.position[i] } else { p.position[i] }
                    });
                    *at = pos;
                    emit(json!({"ev": "pos", "x": pos[0], "y": pos[1], "z": pos[2], "yaw": p.yaw, "pitch": p.pitch}));
                    send(
                        play::Serverbound::AcceptTeleportation(play::AcceptTeleportation {
                            id: p.teleport,
                            x: pos[0],
                            y: pos[1],
                            z: pos[2],
                            yaw: p.yaw,
                            pitch: p.pitch,
                        })
                        .encode(),
                    )
                }
                _ => Ok(()),
            },
            cb::SYSTEM_CHAT | cb::PLAYER_CHAT | cb::DISGUISED_CHAT => {
                match play::Clientbound::decode(id, &body) {
                    Ok(play::Clientbound::SystemChat(p)) => {
                        emit(json!({"ev": "system", "text": plain(&p.content), "overlay": p.overlay}))
                    }
                    Ok(play::Clientbound::PlayerChat(p)) => {
                        let text = p.unsigned_content.as_ref().map_or_else(|| p.content.clone(), plain);
                        emit(json!({"ev": "chat", "sender": format!("{:032x}", p.sender), "text": text}))
                    }
                    Ok(play::Clientbound::DisguisedChat(p)) => {
                        emit(json!({"ev": "disguised", "text": plain(&p.message)}))
                    }
                    _ => {}
                }
                Ok(())
            }
            // Inventories and screens (items area): `slots` for a whole container, `slot` for one, `open`.
            cb::CONTAINER_SET_CONTENT | cb::CONTAINER_SET_SLOT | cb::OPEN_SCREEN => {
                let stack = |s: &Option<horizon_protocol::item::Stack>| s.as_ref().map(|s| json!({"item": s.item, "count": s.count}));
                match play::Clientbound::decode(id, &body) {
                    Ok(play::Clientbound::ContainerSetContent(p)) => emit(json!({"ev": "slots", "container": p.container, "items": p.items.iter().map(stack).collect::<Vec<_>>()})),
                    Ok(play::Clientbound::ContainerSetSlot(p)) => emit(json!({"ev": "slot", "container": p.container, "slot": p.slot, "item": stack(&p.item)})),
                    Ok(play::Clientbound::OpenScreen(p)) => emit(json!({"ev": "open", "container": p.container, "menu": p.menu, "title": plain(&p.title)})),
                    _ => {}
                }
                Ok(())
            }
            cb::DISCONNECT | cb::START_CONFIGURATION => {
                let reason = String::from_utf8_lossy(&body).chars().filter(|c| !c.is_control()).collect::<String>();
                emit(json!({"ev": "disconnect", "reason": reason}));
                std::process::exit(0);
            }
            _ => Ok(()),
        };
        if sent.is_err() {
            emit(json!({"ev": "disconnect", "reason": "send failed"}));
            std::process::exit(0);
        }
    }
}

fn now_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn main() {
    let mut server = "127.0.0.1:25565".to_string();
    let mut name = "Bot".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--server", Some(v)) => server = v,
            ("--name", Some(v)) => name = v,
            _ => {
                eprintln!("usage: ess-bot --server HOST:PORT --name NAME");
                std::process::exit(2);
            }
        }
    }
    let (host, port) = server.rsplit_once(':').unwrap_or((&server, "25565"));
    let port: u16 = port.parse().unwrap_or(25565);
    let connection = match join(host, port, &name) {
        Ok(c) => c,
        Err(e) => {
            emit(json!({"ev": "disconnect", "reason": e}));
            std::process::exit(1);
        }
    };
    let (reader, writer) = connection.split();
    let writer = Arc::new(Mutex::new(writer));
    let position = Arc::new(Mutex::new([0.0; 3]));
    {
        let (writer, position) = (writer.clone(), position.clone());
        thread::spawn(move || read(reader, writer, position));
    }
    use ids::play::serverbound as sb;
    let send = |body: Vec<u8>| lock(&writer).send(&body).is_ok();
    send(raw(sb::PLAYER_LOADED, |_| {}));
    emit(json!({"ev": "joined", "name": name, "uuid": format!("{:032x}", offline_uuid(&name))}));
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(op) = serde_json::from_str::<Value>(&line) else {
            emit(json!({"ev": "error", "reason": format!("not JSON: {line}")}));
            continue;
        };
        let ok = match op["op"].as_str().unwrap_or("") {
            "cmd" => {
                let command = op["line"].as_str().unwrap_or("").trim_start_matches('/').to_string();
                send(play::Serverbound::ChatCommand(play::ChatCommand { command }).encode())
            }
            "chat" => send(
                play::Serverbound::Chat(play::Chat {
                    message: op["text"].as_str().unwrap_or("").to_string(),
                    timestamp: now_millis(),
                    salt: 0,
                    signature: None,
                    last_seen: play::LastSeenUpdate { offset: 0, acknowledged: [0; 3], checksum: 0 },
                })
                .encode(),
            ),
            "move" => {
                let at = [op["x"].as_f64().unwrap_or(0.0), op["y"].as_f64().unwrap_or(0.0), op["z"].as_f64().unwrap_or(0.0)];
                *lock(&position) = at;
                send(play::Serverbound::MovePlayerPos(play::MovePlayerPos { x: at[0], y: at[1], z: at[2], flags: 1 }).encode())
                    && send(raw(sb::CLIENT_TICK_END, |_| {}))
            }
            "close" => send(
                play::Serverbound::ContainerClose(play::ContainerClose { container: op["container"].as_i64().unwrap_or(0) as i32 }).encode(),
            ),
            "quit" => break,
            other => {
                emit(json!({"ev": "error", "reason": format!("unknown op {other}")}));
                true
            }
        };
        if !ok {
            emit(json!({"ev": "disconnect", "reason": "send failed"}));
            std::process::exit(1);
        }
    }
    lock(&writer).shutdown();
}
