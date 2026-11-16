"""Run native mod calculations without compiling or starting Minecraft."""
from pathlib import Path
import json
import os
import shutil
import stat
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
temp_root = Path(tempfile.gettempdir()).resolve(strict=True)
scratch = Path(tempfile.mkdtemp(prefix="atlas-mod-models-", dir=temp_root))
try:
    for source in SOURCES:
        driver = scratch / (source.stem + "-test.rs")
        driver.write_text("#![allow(dead_code)]\nmod game { #[path=" + json.dumps(str(source)) + "] mod subject; }\n", encoding="utf-8")
        output = scratch / (source.stem + (".exe" if os.name == "nt" else ""))
        subprocess.run([rustc, "--edition=2024", "--crate-name", "port_model", "--test", str(driver), "-o", str(output)], check=True, env=env)
        print(source.name, flush=True)
        subprocess.run([str(output)], check=True, env=env)
finally:
    info = scratch.lstat()
    if (scratch.parent != temp_root or not stat.S_ISDIR(info.st_mode)
            or getattr(info, "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT
            or scratch.resolve(strict=True) != scratch):
        raise RuntimeError(f"Unsafe scratch path: {scratch}")
    shutil.rmtree(scratch)
