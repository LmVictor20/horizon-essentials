#!/usr/bin/env python3
"""Packages horizon-essentials into Horizon native mod (.hnmod) archives.

A native mod bundle (.hnmod) is a zip archive checked by horizon-mods (schema 1):
  - horizon-native.json at the zip root
  - all source code under mixins/<id>/
  - strictly only .rs and .toml files beneath mixins/<id>/
  - <= 512 entries, <= 64 MiB expanded size
  - manifest id matching [a-z0-9_]{1,64}

Usage:
  python3 tools/pack.py                     # builds the single horizon_essentials.hnmod in dist/
  python3 tools/pack.py --versioned         # outputs horizon_essentials-<version>.hnmod
  python3 tools/pack.py --verify-engine     # also verify with Rust horizon-mods::index::native
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
COMPAT_FILE = ROOT / "compatibility.json"
MIXINS_DIR = ROOT / "mixins"

MAX_ENTRIES = 512
MAX_EXPANDED_BYTES = 64 * 1024 * 1024  # 64 MiB


def load_compatibility():
    if not COMPAT_FILE.exists():
        raise FileNotFoundError(f"Missing compatibility.json at {COMPAT_FILE}")
    data = json.loads(COMPAT_FILE.read_text(encoding="utf-8"))
    horizon = data.get("horizon_revision", "")
    mixins = data.get("horizon_mixins_revision", "")
    if not is_revision(horizon):
        raise ValueError(f"Invalid horizon_revision in compatibility.json: {horizon}")
    if not is_revision(mixins):
        raise ValueError(f"Invalid horizon_mixins_revision in compatibility.json: {mixins}")
    return data


def is_revision(val: str) -> bool:
    return len(val) == 40 and all(c in "0123456789abcdef" for c in val.lower())


def valid_id(mod_id: str) -> bool:
    """Matches [a-z0-9_]{1,64} from horizon_mods::index::valid_id."""
    return 1 <= len(mod_id) <= 64 and all(c.isascii() and (c.islower() or c.isdigit() or c == "_") for c in mod_id)


def normalize_id(name: str) -> str:
    """Normalizes kebab-case mod IDs (e.g. horizon-essentials) to snake_case (horizon_essentials)."""
    normalized = name.lower().replace("-", "_")
    if not valid_id(normalized):
        raise ValueError(f"Normalized ID '{normalized}' is not valid [a-z0-9_]{{1,64}}")
    return normalized


def toml_val(v) -> str:
    """Formats a value for simple TOML serialization."""
    if isinstance(v, bool):
        return "true" if v else "false"
    elif isinstance(v, int):
        return str(v)
    elif isinstance(v, str):
        escaped = v.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
        return f'"{escaped}"'
    elif isinstance(v, list):
        items = [toml_val(x) for x in v]
        return "[" + ", ".join(items) + "]"
    elif isinstance(v, dict):
        parts = [f"{k} = {toml_val(val)}" for k, val in v.items()]
        return "{ " + ", ".join(parts) + " }"
    raise TypeError(f"Unsupported TOML value type: {type(v)}")


def dump_toml(data: dict) -> str:
    """Produces clean TOML for horizon-mixin.toml."""
    lines = []
    lines.append("[mod]")
    for k, v in data["mod"].items():
        lines.append(f"{k} = {toml_val(v)}")
    lines.append("")
    lines.append("[compatibility]")
    for k, v in data["compatibility"].items():
        lines.append(f"{k} = {toml_val(v)}")
    lines.append("")
    for st in data.get("state", []):
        lines.append("[[state]]")
        for k, v in st.items():
            lines.append(f"{k} = {toml_val(v)}")
        lines.append("")
    for m in data.get("module", []):
        lines.append("[[module]]")
        for k, v in m.items():
            lines.append(f"{k} = {toml_val(v)}")
        lines.append("")
    for inj in data.get("injection", []):
        lines.append("[[injection]]")
        for k, v in inj.items():
            lines.append(f"{k} = {toml_val(v)}")
        lines.append("")
    return "\n".join(lines).strip() + "\n"


def verify_bundle(path: Path) -> dict:
    """Implements every validation check of horizon_mods::index::native::Bundle::read."""
    if not path.is_file():
        raise ValueError(f"Bundle file does not exist: {path}")

    with zipfile.ZipFile(path, "r") as zf:
        infolist = zf.infolist()
        if len(infolist) > MAX_ENTRIES:
            raise ValueError(f"Native bundle has too many entries ({len(infolist)} > {MAX_ENTRIES})")

        total_bytes = 0
        files = {}
        for entry in infolist:
            name = entry.filename
            if entry.is_dir():
                continue

            # Path format checks: no \, no leading /, no :, no . or .. segments
            if "\\" in name or name.startswith("/") or ":" in name:
                raise ValueError(f"Invalid native bundle path: {name}")
            parts = name.split("/")
            if any(p == "" or p == "." or p == ".." for p in parts):
                raise ValueError(f"Invalid native bundle path: {name}")

            content = zf.read(entry)
            total_bytes += len(content)
            if total_bytes > MAX_EXPANDED_BYTES:
                raise ValueError(f"Native bundle expands beyond 64 MiB ({total_bytes} bytes)")

            if name in files:
                raise ValueError(f"Duplicate native bundle path: {name}")
            files[name] = content

    if "horizon-native.json" not in files:
        raise ValueError("Missing horizon-native.json in bundle root")

    try:
        manifest = json.loads(files["horizon-native.json"].decode("utf-8"))
    except Exception as e:
        raise ValueError(f"Corrupt horizon-native.json: {e}")

    # Check manifest fields
    schema = manifest.get("schema")
    mod_id = manifest.get("id", "")
    version = manifest.get("version", "")
    game = manifest.get("game", "")
    horizon = manifest.get("horizon", "")
    mixins = manifest.get("mixins", "")
    hooks = manifest.get("hooks")

    if schema != 1 or not valid_id(mod_id) or not version or not game:
        raise ValueError(
            f"Invalid native manifest: schema 1, valid id [a-z0-9_]{{1,64}}, version and game required. Got: {manifest}"
        )
    if not is_revision(horizon) or not is_revision(mixins):
        raise ValueError(f"Exact 40-hex revisions required for horizon and mixins. Got: {horizon}, {mixins}")
    if hooks is not None and hooks == 0:
        raise ValueError("Manifest hooks cannot be 0")

    # Check all other entries
    prefix = f"mixins/{mod_id}/"
    for name in files:
        if name == "horizon-native.json":
            continue
        if not name.startswith(prefix):
            raise ValueError(f"Unexpected native source path not under {prefix}: {name}")
        if not (name.endswith(".rs") or name.endswith(".toml")):
            raise ValueError(f"Unexpected non-source file in native bundle: {name}")

    # Check horizon-mixin.toml
    mixin_toml_path = f"{prefix}horizon-mixin.toml"
    if mixin_toml_path not in files:
        raise ValueError(f"Missing {mixin_toml_path} in native bundle")

    try:
        mixin_toml = tomllib.loads(files[mixin_toml_path].decode("utf-8"))
    except Exception as e:
        raise ValueError(f"Corrupt {mixin_toml_path}: {e}")

    toml_mod_id = mixin_toml.get("mod", {}).get("id")
    toml_version = mixin_toml.get("mod", {}).get("version")
    toml_horizon = mixin_toml.get("compatibility", {}).get("horizon_revision")

    if toml_mod_id != mod_id or toml_version != version or toml_horizon != horizon:
        raise ValueError(
            f"Native manifest and mixin identity/revision differ: manifest=({mod_id}, {version}, {horizon}), "
            f"mixin=({toml_mod_id}, {toml_version}, {toml_horizon})"
        )

    return {
        "id": mod_id,
        "version": version,
        "game": game,
        "horizon": horizon,
        "mixins": mixins,
        "hooks": hooks,
        "files_count": len(files),
        "total_uncompressed_bytes": total_bytes,
        "archive_size": path.stat().st_size,
    }


def package_modular(out_dir: Path, area_name: str, compat: dict) -> Path:
    """Packages a single mixin area folder into a modular .hnmod."""
    area_dir = MIXINS_DIR / area_name
    if not area_dir.is_dir():
        raise FileNotFoundError(f"Area directory not found: {area_dir}")

    toml_file = area_dir / "horizon-mixin.toml"
    if not toml_file.is_file():
        raise FileNotFoundError(f"Missing horizon-mixin.toml in {area_dir}")

    parsed_toml = tomllib.loads(toml_file.read_text(encoding="utf-8"))
    raw_id = parsed_toml.get("mod", {}).get("id", f"horizon-essentials-{area_name}")
    mod_id = normalize_id(raw_id)
    version = parsed_toml.get("mod", {}).get("version", "0.1.0")

    manifest = {
        "schema": 1,
        "id": mod_id,
        "version": version,
        "game": "26.3",
        "horizon": compat["horizon_revision"],
        "mixins": compat["horizon_mixins_revision"],
    }

    # Update ID in toml content for zip
    updated_toml = parsed_toml.copy()
    updated_toml["mod"]["id"] = mod_id
    updated_toml["compatibility"]["horizon_revision"] = compat["horizon_revision"]
    toml_content = dump_toml(updated_toml).encode("utf-8")

    out_file = out_dir / f"{mod_id}-{version}.hnmod"
    out_dir.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(out_file, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("horizon-native.json", json.dumps(manifest, indent=2) + "\n")
        zf.writestr(f"mixins/{mod_id}/horizon-mixin.toml", toml_content)

        for item in sorted(area_dir.iterdir()):
            if item.is_file() and item.name != "horizon-mixin.toml":
                if item.suffix in (".rs", ".toml") and not item.name.startswith("."):
                    zf.writestr(f"mixins/{mod_id}/{item.name}", item.read_bytes())

    # Convenience link/copy without version
    symlink_file = out_dir / f"{mod_id}.hnmod"
    if symlink_file.exists():
        symlink_file.unlink()
    shutil.copyfile(out_file, symlink_file)

    return out_file


def package_all_in_one(out_dir: Path, compat: dict, versioned: bool = False) -> Path:
    """Packages all areas into a single unified horizon_essentials.hnmod."""
    mod_id = "horizon_essentials"
    core_toml = MIXINS_DIR / "essentials/horizon-mixin.toml"
    parsed_core = tomllib.loads(core_toml.read_text(encoding="utf-8"))
    version = parsed_core.get("mod", {}).get("version", "0.1.0")

    manifest = {
        "schema": 1,
        "id": mod_id,
        "version": version,
        "game": "26.3",
        "horizon": compat["horizon_revision"],
        "mixins": compat["horizon_mixins_revision"],
    }

    # Order of areas to assemble
    ordered_areas = [
        "essentials",
        "ess-tp",
        "ess-social",
        "ess-admin",
        "ess-items",
        "ess-economy",
        "ess-world",
    ]

    combined_modules = []
    combined_injections = []
    combined_states = []
    files_to_pack = {}  # target_filename -> bytes

    for area in ordered_areas:
        area_dir = MIXINS_DIR / area
        if not area_dir.is_dir():
            continue

        toml_data = tomllib.loads((area_dir / "horizon-mixin.toml").read_text(encoding="utf-8"))
        for m in toml_data.get("module", []):
            combined_modules.append(m)
        for st in toml_data.get("state", []):
            combined_states.append(st)

        for inj in toml_data.get("injection", []):
            # Resolve tick hook collisions
            if inj["id"] == "tick":
                if area == "ess-economy":
                    inj["id"] = "tick-economy"
                    inj["helper"] = "hook_economy_tick.rs"
                    inj["priority"] = 90
                elif area == "ess-social":
                    inj["id"] = "tick-social"
                    inj["helper"] = "hook_social_tick.rs"
                    inj["priority"] = 80
                else:
                    inj["priority"] = 100
            elif inj["id"] == "world-tick":
                inj["priority"] = 70

            combined_injections.append(inj)

        # Collect source files
        for f in sorted(area_dir.iterdir()):
            if f.is_file() and f.suffix in (".rs", ".toml") and f.name != "horizon-mixin.toml":
                target_name = f.name
                if f.name == "hook_tick.rs":
                    if area == "ess-economy":
                        target_name = "hook_economy_tick.rs"
                    elif area == "ess-social":
                        target_name = "hook_social_tick.rs"
                files_to_pack[target_name] = f.read_bytes()

    combined_toml = {
        "mod": {
            "id": mod_id,
            "version": version,
            "priority": 100,
        },
        "compatibility": {
            "horizon_revision": compat["horizon_revision"],
        },
        "module": combined_modules,
        "injection": combined_injections,
    }
    if combined_states:
        combined_toml["state"] = combined_states

    combined_toml_bytes = dump_toml(combined_toml).encode("utf-8")

    out_name = f"{mod_id}-{version}.hnmod" if versioned else f"{mod_id}.hnmod"
    out_file = out_dir / out_name
    out_dir.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(out_file, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("horizon-native.json", json.dumps(manifest, indent=2) + "\n")
        zf.writestr(f"mixins/{mod_id}/horizon-mixin.toml", combined_toml_bytes)

        for filename, content in sorted(files_to_pack.items()):
            zf.writestr(f"mixins/{mod_id}/{filename}", content)

    return out_file


def verify_with_rust_engine(hnmod_path: Path) -> bool:
    """Verifies the bundle using horizon-mods' Bundle::read Rust implementation."""
    cargo_toml = ROOT.parent / "horizon/crates/horizon-mods/Cargo.toml"
    if not cargo_toml.exists():
        print("Note: horizon checkout not found beside repo; skipping Rust engine verification.")
        return True

    rust_code = f"""
use std::path::Path;
fn main() {{
    let path = Path::new({json.dumps(str(hnmod_path))});
    match horizon_mods::index::native::Bundle::read(path) {{
        Ok(bundle) => {{
            println!("Bundle validated by horizon_mods: id={{}}, files={{}}", bundle.manifest.id, bundle.files.len());
        }}
        Err(e) => {{
            eprintln!("horizon_mods rejection: {{}}", e);
            std::process::exit(1);
        }}
    }}
}}
"""
    with tempfile.TemporaryDirectory() as td:
        src_path = Path(td) / "test_read.rs"
        src_path.write_text(rust_code)
        manifest_path = Path(td) / "Cargo.toml"
        manifest_path.write_text(f"""[package]
name = "hnmod-validator"
version = "0.1.0"
edition = "2024"

[dependencies]
horizon-mods = {{ path = {json.dumps(str(cargo_toml.parent))}, features = ["index"] }}

[[bin]]
name = "validate"
path = {json.dumps(str(src_path))}
""")
        cmd = ["cargo", "run", "--quiet", "--manifest-path", str(manifest_path)]
        res = subprocess.run(cmd, capture_output=True, text=True)
        if res.returncode != 0:
            print(f"Rust engine verification failed for {hnmod_path.name}:", file=sys.stderr)
            print(res.stderr, file=sys.stderr)
            return False
        print(f"  [Rust Engine OK] {res.stdout.strip()}")
        return True


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out-dir", type=Path, default=ROOT / "dist", help="Output directory (default: dist/)")
    parser.add_argument("--versioned", action="store_true", help="Include version in filename (e.g. horizon_essentials-0.1.0.hnmod)")
    parser.add_argument("--modular", action="store_true", help="Build modular area bundles in dist/modular/")
    parser.add_argument("--bundle", type=str, help="Build only a specific area bundle (e.g. tp, social, essentials)")
    parser.add_argument("--no-verify", action="store_true", help="Skip bundle validation")
    parser.add_argument("--verify-engine", action="store_true", help="Run validation using Rust horizon-mods")
    parser.add_argument("--clean", action="store_true", help="Clean output directory before packaging")
    args = parser.parse_args()

    compat = load_compatibility()
    out_dir = args.out_dir.resolve()

    if args.clean and out_dir.exists():
        shutil.rmtree(out_dir)

    # If building single mod (default), remove any existing dist/modular to keep dist/ clean
    if not args.modular and not args.bundle:
        modular_dir = out_dir / "modular"
        if modular_dir.exists():
            shutil.rmtree(modular_dir)

    out_dir.mkdir(parents=True, exist_ok=True)
    built = []

    # Map aliases for --bundle
    area_map = {
        "core": "essentials",
        "essentials": "essentials",
        "admin": "ess-admin",
        "ess-admin": "ess-admin",
        "economy": "ess-economy",
        "ess-economy": "ess-economy",
        "items": "ess-items",
        "ess-items": "ess-items",
        "social": "ess-social",
        "ess-social": "ess-social",
        "tp": "ess-tp",
        "ess-tp": "ess-tp",
        "world": "ess-world",
        "ess-world": "ess-world",
    }

    if args.bundle:
        key = args.bundle.lower()
        if key not in area_map:
            parser.error(f"Unknown bundle '{args.bundle}'. Available: {sorted(set(area_map.keys()))}")
        modular_dir = out_dir / "modular"
        path = package_modular(modular_dir, area_map[key], compat)
        built.append(path)
    elif args.modular:
        modular_dir = out_dir / "modular"
        areas = sorted([d.name for d in MIXINS_DIR.iterdir() if d.is_dir()])
        for area in areas:
            path = package_modular(modular_dir, area, compat)
            built.append(path)
    else:
        # Default: build one single unified mod file (horizon_essentials.hnmod)
        path = package_all_in_one(out_dir, compat, versioned=args.versioned)
        built.append(path)

    print(f"\nSuccessfully generated {len(built)} .hnmod packages:")
    all_ok = True
    for p in built:
        rel = p.relative_to(ROOT) if p.is_relative_to(ROOT) else p
        size_kb = p.stat().st_size / 1024
        print(f"  * {rel} ({size_kb:.1f} KiB)")

        if not args.no_verify:
            try:
                facts = verify_bundle(p)
                print(
                    f"    [Verified] id={facts['id']}, files={facts['files_count']}, "
                    f"uncompressed={facts['total_uncompressed_bytes'] / 1024:.1f} KiB"
                )
                if args.verify_engine:
                    if not verify_with_rust_engine(p):
                        all_ok = False
            except Exception as e:
                print(f"    [VERIFICATION ERROR] {e}", file=sys.stderr)
                all_ok = False

    if not all_ok:
        sys.exit(1)


if __name__ == "__main__":
    try:
        main()
    except Exception as err:
        print(f"Error: {err}", file=sys.stderr)
        sys.exit(1)
