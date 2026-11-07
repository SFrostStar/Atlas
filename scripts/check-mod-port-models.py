"""Run native mod calculations without compiling or starting Minecraft."""
from pathlib import Path
import json
import os
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PORTS = ROOT / "engine/atlas/crates/atlas-client/src/game/atlas_ports"
SOURCES = [PORTS / name for name in (
    "food_logic.rs", "utility_logic.rs", "actions_logic.rs",
    "actions_viewmodel.rs", "actions_inventory_planner.rs", "visual_logic.rs",
)] + [ROOT / "engine/atlas/crates/atlas-client-render/src/atlas/chunk_animation.rs"]
env = os.environ.copy()
portable = ROOT / "tools/cargo/bin/rustc.exe"
rustc = str(portable) if portable.exists() else shutil.which("rustc")
if not rustc:
    raise SystemExit("Rust is required")
if portable.exists():
    env["CARGO_HOME"] = str(ROOT / "tools/cargo")
    env["RUSTUP_HOME"] = str(ROOT / "tools/rustup")
    llvm = next((ROOT / "tools").glob("llvm-mingw-*-ucrt-x86_64"))
    env["PATH"] = os.pathsep.join((str(llvm / "bin"), str(portable.parent), env.get("PATH", "")))
(ROOT / "work").mkdir(exist_ok=True)
scratch = Path(tempfile.mkdtemp(prefix="mod-models-", dir=ROOT / "work"))
for source in SOURCES:
    driver = scratch / (source.stem + "-test.rs")
    driver.write_text("#![allow(dead_code)]\nmod game { #[path=" + json.dumps(str(source)) + "] mod subject; }\n", encoding="utf-8")
    output = scratch / (source.stem + (".exe" if os.name == "nt" else ""))
    subprocess.run([rustc, "--edition=2024", "--crate-name", "port_model", "--test", str(driver), "-o", str(output)], check=True, env=env)
    print(source.name, flush=True)
    subprocess.run([str(output)], check=True, env=env)
