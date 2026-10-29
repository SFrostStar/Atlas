"""Check the Git index before a commit/push. Never print file contents or secrets."""
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PRIVATE_DIRS = {"data", "work", "runtime", "node_modules", "target", ".local",
                "saves", "worlds", "backups", "screenshots", "crash-reports",
                "logs", "run", "minecraft", "decompiled", "auth-sessions",
                ".atlas-cache", ".horizon-cache"}
PRIVATE_NAMES = {"options.txt", "optionsof.txt", "accounts.json", "launcher.json",
                 "session.json", "control.json", "credentials.json", "tokens.json",
                 "servers.dat", "usercache.json", "ops.json", "whitelist.json",
                 "banned-players.json", "banned-ips.json", "server.properties"}
PRIVATE_SUFFIXES = {".mca", ".mcr", ".dat", ".dat_old", ".jar", ".class", ".jfr",
                    ".hprof", ".log", ".pem", ".key", ".pfx", ".p12", ".keystore",
                    ".exe", ".dll", ".wasm", ".zip", ".7z", ".rar", ".lnk"}
LOCAL_ICONS = {"assets/game-icon.png", "assets/game-icon.ico",
               "engine/atlas/packaging/icons/atlas.ico"}
SECRET = re.compile(rb"(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)")


def prohibited(name):
    path = PurePosixPath(name)
    return (any(p in PRIVATE_DIRS for p in path.parts)
            or (path.parts and path.parts[0] == "tools")
            or path.name in PRIVATE_NAMES or path.suffix.lower() in PRIVATE_SUFFIXES
            or (path.name.startswith(".env") and path.name != ".env.example")
            or re.fullmatch(r"(?:atlas|horizon)-(?:extras|graphics|player|controller)\.txt", path.name)
            or path.name in {"atlas-shaders.toml", "horizon-shaders.toml"}
            or name in LOCAL_ICONS)


def main():
    entries = subprocess.check_output(["git", "ls-files", "--stage", "-z"], cwd=ROOT).decode().split("\0")
    rejected = []
    checked = 0
    objects = subprocess.Popen(["git", "cat-file", "--batch"], cwd=ROOT,
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    for entry in filter(None, entries):
        metadata, name = entry.split("\t", 1)
        mode, oid, stage = metadata.split()
        reason = None
        if mode == "160000":
            reason = "embedded Git repository: source files must be included explicitly"
        elif prohibited(name):
            reason = "private/local artifact path"
        else:
            objects.stdin.write((oid + "\n").encode())
            objects.stdin.flush()
            header = objects.stdout.readline().split()
            if len(header) != 3 or header[1] != b"blob":
                raise RuntimeError("Unable to inspect indexed Git object")
            content = objects.stdout.read(int(header[2]))
            objects.stdout.read(1)
            if len(content) > 50 * 1024 * 1024:
                reason = "unexpectedly large source artifact"
            elif SECRET.search(content):
                reason = "credential/private-key pattern"
            elif (b"C:/" + b"Users/" + Path.home().name.encode()) in content or (b"C:\\" + b"Users\\" + Path.home().name.encode()) in content:
                reason = "machine-specific user path"
        if reason:
            rejected.append((name, reason))
        checked += 1
    objects.stdin.close()
    objects.wait()
    if rejected:
        for name, reason in rejected:
            print(f"BLOCKED {name}: {reason}", file=sys.stderr)
        return 1
    print(f"Privacy check passed: {checked} indexed source files; no blocked paths or credential patterns.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
