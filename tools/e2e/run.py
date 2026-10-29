#!/usr/bin/env python3
"""End-to-end tests (L3): a real Horizon dedicated server with horizon-essentials, and ess-bot players.

  python3 tools/e2e/run.py                      # every scenario in tools/e2e/scenarios/
  python3 tools/e2e/run.py ping teleport.tpa    # a file, or file.function
  python3 tools/e2e/run.py --server-bin PATH    # a server already built (default: tools/horizon.py build's)

A scenario file holds `def test_<name>(t)` functions. Each gets a fresh server folder (world, essentials
data) and a running server; `t` gives:
  t.bot("Alice")            join a player (offline mode); returns a Bot
  t.console("op Alice")     a console command; t.wait_log("regex") waits for a server log line
  t.server_dir              the server folder (essentials/ data lives under it)
  bot.cmd("/home base")     send a command;  bot.chat("hi")  send chat;  bot.move(x, y, z)
  bot.expect("regex")       wait for a system/chat line matching; returns it (default timeout 10 s)
  bot.expect_none("regex", seconds=2)   no such line within the time
  bot.expect_pos(x, y, z, within=1.5)   wait for a teleport near there
  bot.lines                 every event so far (dicts from ess-bot)
Players and the server use scratch homes; nothing touches ~/.local/share/horizon*.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import queue
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import traceback

ROOT = Path(__file__).resolve().parents[2]
SCENARIOS = ROOT / "tools/e2e/scenarios"
DEFAULT_RESOURCES = Path.home() / ".local/share/horizon-launcher/server-runtime/game-files/horizon-client/26.3"
DEFAULT_SERVER = ROOT / "target/horizon-cargo/fast/horizon-client"
BOT = ROOT / "target/ess-bot/debug/ess-bot"

PROPERTIES = """\
online-mode=false
enforce-secure-profile=false
white-list=false
spawn-protection=0
view-distance=3
simulation-distance=3
level-seed=12345
level-type=minecraft\\:flat
generate-structures=false
spawn-monsters=false
server-port={port}
motd=horizon-essentials e2e
"""


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Lines:
    """Lines from a process, kept and also handed to waiters."""

    def __init__(self, stream, parse):
        self.all, self.queue = [], queue.Queue()
        self.parse = parse
        threading.Thread(target=self._pump, args=(stream,), daemon=True).start()

    def _pump(self, stream):
        for raw in stream:
            item = self.parse(raw.rstrip("\n"))
            if item is not None:
                self.all.append(item)
                self.queue.put(item)
        self.queue.put(None)

    def wait(self, match, timeout):
        deadline = time.monotonic() + timeout
        while True:
            left = deadline - time.monotonic()
            if left <= 0:
                return None
            try:
                item = self.queue.get(timeout=left)
            except queue.Empty:
                return None
            if item is None:
                return None
            if match(item):
                return item


class Bot:
    def __init__(self, name, port, home):
        self.name = name
        self.proc = subprocess.Popen([str(BOT), "--server", f"127.0.0.1:{port}", "--name", name],
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                     text=True, cwd=home)

        def parse(line):
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                return {"ev": "stderr", "text": line}

        self.events = Lines(self.proc.stdout, parse)
        joined = self.events.wait(lambda e: e.get("ev") in ("joined", "disconnect"), 30)
        if not joined or joined["ev"] != "joined":
            raise AssertionError(f"{name} could not join: {joined}")
        self.uuid = joined["uuid"]
        # The server places a player a moment after login: wait for the first position.
        self.expect_event(lambda e: e.get("ev") == "pos", 30, "the spawn position")

    @property
    def lines(self):
        return list(self.events.all)

    def _send(self, op):
        self.proc.stdin.write(json.dumps(op) + "\n")
        self.proc.stdin.flush()

    def cmd(self, line):
        self._send({"op": "cmd", "line": line})

    def chat(self, text):
        self._send({"op": "chat", "text": text})

    def move(self, x, y, z):
        self._send({"op": "move", "x": x, "y": y, "z": z})

    def expect_event(self, match, timeout, what):
        event = self.events.wait(match, timeout)
        if event is None:
            seen = "\n  ".join(json.dumps(e) for e in self.events.all[-15:])
            raise AssertionError(f"{self.name}: no {what} within {timeout} s; last events:\n  {seen}")
        return event

    def expect(self, pattern, timeout=10):
        rx = re.compile(pattern)
        return self.expect_event(lambda e: e.get("ev") in ("system", "chat", "disguised")
                                 and rx.search(e.get("text", "")), timeout, f"line matching {pattern!r}")["text"]

    def expect_none(self, pattern, seconds=2):
        rx = re.compile(pattern)
        hit = self.events.wait(lambda e: e.get("ev") in ("system", "chat", "disguised")
                               and rx.search(e.get("text", "")), seconds)
        if hit:
            raise AssertionError(f"{self.name}: unexpected line {hit['text']!r}")

    def expect_pos(self, x, y, z, within=1.5, timeout=10):
        def near(e):
            return e.get("ev") == "pos" and all(abs(e[k] - v) <= within for k, v in (("x", x), ("y", y), ("z", z)))
        return self.expect_event(near, timeout, f"teleport near {x} {y} {z}")

    def quit(self):
        if self.proc.poll() is None:
            try:
                self._send({"op": "quit"})
                self.proc.wait(5)
            except (OSError, subprocess.TimeoutExpired):
                self.proc.kill()


class Server:
    def __init__(self, binary, resources, scratch):
        self.port = free_port()
        self.dir = scratch / "server"
        self.dir.mkdir()
        (self.dir / "eula.txt").write_text("eula=true\n")
        (self.dir / "server.properties").write_text(PROPERTIES.format(port=self.port))
        home = scratch / "home"
        env = dict(os.environ, HORIZON_HOME=str(home / "horizon"), HORIZON_NO_AUDIO="1",
                   XDG_DATA_HOME=str(home / "data"), XDG_CONFIG_HOME=str(home / "config"),
                   HORIZON_NO_UPDATE_CHECK="1")
        self.proc = subprocess.Popen([str(binary), "--resources", str(resources), "--dedicated", "--nogui",
                                      "--port", str(self.port)], cwd=self.dir, env=env, text=True,
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.log = Lines(self.proc.stdout, lambda line: line)
        if not self.log.wait(lambda line: "Done (" in line, 180):
            tail = "\n  ".join(self.log.all[-25:])
            raise AssertionError(f"the server did not start; last lines:\n  {tail}")

    def console(self, line):
        self.proc.stdin.write(line.lstrip("/") + "\n")
        self.proc.stdin.flush()

    def wait_log(self, pattern, timeout=10):
        rx = re.compile(pattern)
        line = self.log.wait(lambda l: rx.search(l), timeout)
        if line is None:
            raise AssertionError(f"no server log line matching {pattern!r} within {timeout} s")
        return line

    def stop(self):
        if self.proc.poll() is None:
            try:
                self.console("stop")
                self.proc.wait(60)
            except (OSError, subprocess.TimeoutExpired):
                self.proc.kill()


class Test:
    def __init__(self, server, scratch):
        self.server, self.scratch, self.bots = server, scratch, []
        self.server_dir = server.dir

    def bot(self, name):
        home = self.scratch / f"bot-{name}"
        home.mkdir(exist_ok=True)
        bot = Bot(name, self.server.port, home)
        self.bots.append(bot)
        return bot

    def console(self, line):
        self.server.console(line)

    def wait_log(self, pattern, timeout=10):
        return self.server.wait_log(pattern, timeout)


def collect(selection):
    tests = []
    files = sorted(SCENARIOS.glob("*.py"))
    for file in files:
        spec = importlib.util.spec_from_file_location(f"scenario_{file.stem}", file)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        for name in sorted(n for n in dir(module) if n.startswith("test_")):
            key = f"{file.stem}.{name[5:]}"
            if not selection or any(key == s or file.stem == s or key.startswith(s + ".") or file.stem.startswith(s + "_")
                                    for s in selection):
                tests.append((key, getattr(module, name)))
    return tests


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("select", nargs="*")
    parser.add_argument("--server-bin", type=Path, default=DEFAULT_SERVER)
    parser.add_argument("--resources", type=Path, default=Path(os.environ.get("HORIZON_RESOURCES", DEFAULT_RESOURCES)))
    parser.add_argument("--keep", action="store_true", help="keep the scratch folders of failed tests")
    args = parser.parse_args()
    for path, what in ((args.server_bin, "server (python3 tools/horizon.py build)"),
                       (BOT, "ess-bot (cargo build --manifest-path tools/ess-bot/Cargo.toml --target-dir target/ess-bot)"),
                       (args.resources / "registries.json", "prepared game files (--resources)")):
        if not path.exists():
            parser.error(f"missing {what}: {path}")
    tests = collect(args.select)
    if not tests:
        parser.error("no scenario matches")
    failed = []
    for key, test in tests:
        scratch = Path(tempfile.mkdtemp(prefix=f"ess-e2e-{key}-"))
        started = time.monotonic()
        server = None
        t = None
        try:
            server = Server(args.server_bin, args.resources, scratch)
            t = Test(server, scratch)
            test(t)
            print(f"PASS {key} ({time.monotonic() - started:.1f} s)", flush=True)
            ok = True
        except Exception:
            print(f"FAIL {key}\n{traceback.format_exc()}", flush=True)
            failed.append(key)
            ok = False
        finally:
            for bot in (t.bots if t else []):
                bot.quit()
            if server:
                server.stop()
                if not ok:
                    print("  server log tail:\n    " + "\n    ".join(server.log.all[-20:]), flush=True)
            if ok or not args.keep:
                shutil.rmtree(scratch, ignore_errors=True)
            else:
                print(f"  kept {scratch}", flush=True)
    print(f"{len(tests) - len(failed)} passed, {len(failed)} failed" + (f": {', '.join(failed)}" if failed else ""))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
