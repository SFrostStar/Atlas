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
RENDER = ROOT / "engine/atlas/crates/atlas-client-render/src"
SOURCES = [PORTS / name for name in (
    "food_logic.rs", "utility_logic.rs", "actions_logic.rs",
    "visual_logic.rs",
)] + [RENDER / "atlas/chunk_animation.rs"]
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
        flags = []
        if source.name == "visual_logic.rs":
            motion = scratch / "motion.rs"
            motion_lib = scratch / "libatlas_client.rlib"
            motion_path = json.dumps(str(RENDER / "hud_kit/motion.rs"))
            motion.write_text(f"#[path={motion_path}] pub mod motion;\npub mod hud_kit {{ pub use crate::motion; }}\n", encoding="utf-8")
            subprocess.run([rustc, "--edition=2024", "--crate-name", "atlas_client", "--crate-type", "rlib", str(motion), "-o", str(motion_lib)], check=True, env=env)
            flags = ["--extern", f"atlas_client={motion_lib}"]
        driver.write_text("#![allow(dead_code)]\nmod game { #[path=" + json.dumps(str(source)) + "] mod subject; }\n", encoding="utf-8")
        output = scratch / (source.stem + (".exe" if os.name == "nt" else ""))
        subprocess.run([rustc, "--edition=2024", "--crate-name", "port_model", "--test", *flags, str(driver), "-o", str(output)], check=True, env=env)
        print(source.name, flush=True)
        subprocess.run([str(output)], check=True, env=env)
finally:
    info = scratch.lstat()
    if (scratch.parent != temp_root or not stat.S_ISDIR(info.st_mode)
            or getattr(info, "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT
            or scratch.resolve(strict=True) != scratch):
        raise RuntimeError(f"Unsafe scratch path: {scratch}")
    shutil.rmtree(scratch)
