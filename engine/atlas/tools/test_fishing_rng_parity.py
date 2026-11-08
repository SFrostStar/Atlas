#!/usr/bin/env python3
"""Compare fishing movement and retrieval against a real 26.3 server and player.

The Java side calls FishingHook.retrieve with seeded hook and table sequences.
The Rust side compares seeded cast velocity, bobbing and dry-ground ticks, catch-timer
transitions and the rod's fallback loot path, plus named-sequence loot and XP.
It does not yet cover the UUID-to-seed relationship.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

try:
    from . import lab
except ImportError:
    import lab


def main():
    work = Path(tempfile.mkdtemp(prefix="fishing-rng-", dir=lab.LOCAL))
    java_options = os.environ.get("JAVA_TOOL_OPTIONS", "")
    env = lab.game_env(
        JAVA_TOOL_OPTIONS=java_options + " -Datlas.fishingReference=true",
        CARGO_TARGET_DIR=str(lab.cargo_target()),
        ATLAS_FISHING_TICK_REFERENCE=str(work / "rust-fishing-ticks.json"),
        ATLAS_FISHING_BOBBING_REFERENCE=str(work / "rust-fishing-bobbing.json"),
        ATLAS_HOME=str(work / "home"),
        ATLAS_LAUNCHER_HOME=str(work / "launcher"),
        XDG_DATA_HOME=str(work / "data"),
        XDG_CONFIG_HOME=str(work / "config"),
    )
    subprocess.run(["cargo", "build", "--profile", "fast", "-p", "atlas-client", "--bin", "atlas-client"],
                   cwd=lab.ROOT, env=env, check=True)
    binary = lab.cargo_target() / "fast" / ("atlas-client.exe" if os.name == "nt" else "atlas-client")
    subprocess.run([sys.executable, str(lab.ROOT / "tools/test_command_exec_parity.py"),
                    "--only", "scoreboard", "--allow-gaps", "--binary", str(binary), "--work-dir", str(work)],
                   cwd=lab.ROOT, env=env, check=True)
    reference = json.loads((work / "fishing-ticks-reference.json").read_text())
    actual = json.loads((work / "rust-fishing-ticks.json").read_text())
    if len(reference) != 25 or len(actual) != 25:
        raise SystemExit(f"expected 25 fishing tick cases: {len(reference)} vs {len(actual)}")
    if reference != actual:
        for expected, got in zip(reference, actual):
            if expected != got:
                raise SystemExit(f"fishing tick differs: vanilla {expected}; Atlas {got}")
        raise SystemExit(f"fishing tick record count differs: {len(reference)} vs {len(actual)}")
    expected_bobbing = json.loads((work / "fishing-bobbing-reference.json").read_text())
    actual_bobbing = json.loads((work / "rust-fishing-bobbing.json").read_text())
    if len(expected_bobbing) != 20 or len(actual_bobbing) != 20:
        raise SystemExit(f"expected 20 bobbing and ground cases: {len(expected_bobbing)} vs {len(actual_bobbing)}")
    for expected, actual in zip(expected_bobbing, actual_bobbing):
        if "case" in expected:
            # Collision clipping can differ below a picometre; retain exact
            # case/tick matching and compare all three positions and velocities.
            if expected["case"] != actual.get("case") or expected["tick"] != actual.get("tick"):
                raise SystemExit(f"fishing ground case differs: {expected}; {actual}")
            for field in ("position", "motion"):
                if len(actual[field]) != 3 or any(not abs(a - b) <= 1e-12 for a, b in zip(expected[field], actual[field])):
                    raise SystemExit(f"fishing ground differs: vanilla {expected}; Atlas {actual}")
        elif expected != actual:
            raise SystemExit(f"fishing bobbing differs: vanilla {expected}; Atlas {actual}")
    fallback = work / "fallback"
    fallback_env = env.copy()
    fallback_env.pop("ATLAS_FISHING_TICK_REFERENCE", None)
    fallback_env.pop("ATLAS_FISHING_BOBBING_REFERENCE", None)
    fallback_env["ATLAS_FISHING_FALLBACK_REFERENCE"] = str(fallback / "rust-fishing-fallback.json")
    fallback_env["ATLAS_FISHING_FALLBACK_FIXTURE"] = "1"
    fallback_env["JAVA_TOOL_OPTIONS"] = java_options + " -Datlas.fishingFallback=true"
    subprocess.run([sys.executable, str(lab.ROOT / "tools/test_command_exec_parity.py"),
                    "--only", "scoreboard", "--allow-gaps", "--binary", str(binary), "--work-dir", str(fallback)],
                   cwd=lab.ROOT, env=fallback_env, check=True)
    expected = json.loads((fallback / "fishing-fallback-reference.json").read_text())
    actual = json.loads((fallback / "rust-fishing-fallback.json").read_text())
    if len(expected) != 5 or len(actual) != 5:
        raise SystemExit(f"expected 5 fallback cases: {len(expected)} vs {len(actual)}")
    if expected != actual:
        for reference_row, actual_row in zip(expected, actual):
            if reference_row != actual_row:
                raise SystemExit(f"fishing fallback differs: vanilla {reference_row}; Atlas {actual_row}")
        raise SystemExit(f"fishing fallback record count differs: {len(expected)} vs {len(actual)}")
    env["ATLAS_FISHING_RESOURCES"] = str(lab.RESOURCES)
    env["ATLAS_FISHING_REFERENCE"] = str(work / "fishing-reference.json")
    env["ATLAS_FISHING_CAST_REFERENCE"] = str(work / "fishing-casts-reference.json")
    subprocess.run(["cargo", "test", "--profile", "fast", "-p", "atlas-client", "--lib",
                    "game::fishing::tests::fishing_retrieve_reference", "--", "--exact", "--ignored"],
                   cwd=lab.ROOT, env=env, check=True)
    print("fishing: ground prediction, bobbing, seeded catch timers, loot and XP match vanilla")


if __name__ == "__main__":
    main()
