#!@shell@
# atlas-prepare from Nix: the program wants a writable root (research-lock.json, the exporters'
# sources, .local/ for its scratch files), which the store can't be. Unless --root is given, a copy of
# the packaged root goes to $ATLAS_PREPARE_ROOT, by default ${XDG_DATA_HOME:-~/.local/share}/atlas-nix/prepare.
set -eu
export JAVA_HOME="${JAVA_HOME:-@java@}"
for arg in "$@"; do
  case "$arg" in
    --root | --root=*) exec @out@/libexec/atlas-prepare "$@" ;;
  esac
done
root="${ATLAS_PREPARE_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/atlas-nix/prepare}"
@coreutils@/bin/mkdir -p "$root"
@coreutils@/bin/cp -R --no-preserve=mode @out@/share/atlas/prepare/. "$root/"
exec @out@/libexec/atlas-prepare --root "$root" "$@"
