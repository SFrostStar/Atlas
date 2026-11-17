//! Gives the Windows executable Atlas's icon and its stack, as atlas-client's
//! (`packaging/windows/exe.rs`).
#[path = "../../packaging/windows/exe.rs"]
mod windows_exe;

fn main() {
    windows_exe::resources("atlas-official");
}
