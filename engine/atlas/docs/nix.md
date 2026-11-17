# Nix and NixOS

The repository is a flake (`flake.nix`, and the game's packages in `nix/`). It builds Atlas from source with nixpkgs'
`rustPlatform` and holds none of Mojang's files: the game files still come from `atlas-prepare` and your own
copy of Minecraft 26.3, as everywhere else ([hosting.md](hosting.md), step 2).

| Package | What | Systems |
|---|---|---|
| `atlas-server` | `atlas-server`, with `atlas-engine` (the game without sound or controllers) beside it | Linux, macOS (Apple Silicon) |
| `atlas-engine` | The server build of the game alone | Linux, macOS |
| `atlas-prepare` | `atlas-prepare`, with a JDK 25 for the exporters it compiles | Linux, macOS |
| `atlas-client` | The game and `atlas-ctl` | Linux, macOS |
| `atlas-launcher` | The launcher in an FHS environment (it builds native mods with Cargo) | Linux |

```sh
nix run github:atlas-rust/atlas#atlas-server -- --version
nix profile install github:atlas-rust/atlas#atlas-server
```

`overlays.default` adds them to `pkgs`; on Linux, `nix develop` gives the build environment of the workspace.

## Preparing the game files

`atlas-prepare` works in a writable folder (its pinned hashes, the exporters' sources, scratch files). The
Nix wrapper copies one to `$ATLAS_PREPARE_ROOT`, by default `~/.local/share/atlas-nix/prepare`, unless you
pass `--root`. Write the files where you want them with `--output`:

```sh
atlas-prepare --prepare --server-jar ~/Downloads/server.jar --output ~/atlas/26.3
```

## A server on NixOS: `services.atlas-server`

```nix
{
  inputs.atlas.url = "github:atlas-rust/atlas";

  outputs = { nixpkgs, atlas, ... }: {
    nixosConfigurations.box = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        atlas.nixosModules.atlas-server
        {
          services.atlas-server = {
            enable = true;
            eula = true;                      # you agree to https://aka.ms/MinecraftEULA
            openFirewall = true;              # port 25565 by default
            resources = "/srv/atlas/26.3";  # the folder atlas-prepare wrote, copied here
            init = { world = "survival"; players = [ "Alex" ]; op = "Alex"; };
            settings = { "world.difficulty" = "hard"; "server.max_players" = 10; };
          };
        }
      ];
    };
  };
}
```

- The server folder is `/var/lib/atlas-server` (`dataDir`), owned by the `atlas-server` user. The first
  start writes `server.toml` from `init`, as `atlas-server init` would. Every start then sets `port`,
  `resources` and `settings` with `atlas-server config set`, which checks them and keeps the rest of the
  file: the settings you don't name stay yours to edit.
- Give paths as strings, not Nix paths: a Nix path would copy the game files into the world-readable store.
- The service runs `atlas-server run` and stops it with SIGTERM, which saves the world first. Manage it as
  that user: `sudo -u atlas-server atlas-server --dir /var/lib/atlas-server status` (or `cmd`, `logs`,
  `backup`...).
- Nix decides the release: the update check is off (`ATLAS_NO_UPDATE_CHECK=1`) and `atlas-server update`
  refuses, since this isn't an install.sh install.
- The unit is hardened (read-only system, no home folders, no capabilities, its own `/tmp` and devices). A
  `resources` folder under `/home` isn't visible to it: keep it elsewhere, such as `/srv`.

### Preparing on the server

Instead of `resources`, `prepare` runs `atlas-prepare` before the server starts, from files you put on the
machine yourself: the 26.3 `server.jar` and a copy of the official launcher's game folder with 26.3 played
once (`versions/26.3`, `libraries`, `assets`), readable by the `atlas-server` user.

```nix
services.atlas-server.prepare = {
  enable = true;
  serverJar = "/srv/atlas/server.jar";
  minecraftDir = "/srv/atlas/minecraft";
};
```

It writes `/var/lib/atlas-server/game-files`, and runs again only when the `atlas-prepare` package or
those paths change. The `atlas-server-prepare` unit has no network at all: nothing is ever downloaded.
