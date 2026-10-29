//! Atlas: native Rust client for Minecraft Java Edition 26.3 gameplay.
#![forbid(unsafe_code)]
mod game;

use atlas_client::{crash, logging};

/// The mod API level this game serves, kept in the program for the launcher
/// to read without running it (`atlas_mods::api_level::MARK`). `main`
/// reads it, so the linker keeps it.
static MOD_API_LEVEL: [u8; atlas_mods::api_level::MARK.len()] = atlas_mods::api_level::MARK;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    atlas_client::telemetry::mark_start();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let game_args = args.iter().take_while(|a| *a != "--dedicated");
    let help = game_args.clone().any(|a| a == "--help" || a == "-h");
    // The dedicated server logs in its own folder, as vanilla's does; the game in its home.
    let (home, side, thread) = if args.iter().any(|a| a == "--dedicated") {
        (std::path::PathBuf::from("."), "server", "Server thread")
    } else {
        (logging::game_home(), "client", "Render thread")
    };
    if !help {
        logging::init(&home, thread);
        // The log's first line names the build, so a bug report that carries the log says which one it was.
        log::info!("Atlas {}", crash::version());
        crash::install(home.clone(), side);
    }
    let _memory_limit = match atlas_memory::megabytes(std::env::var("ATLAS_MEMORY_MB").ok().as_deref())
        .and_then(|budget| budget.map(atlas_memory::apply).transpose())
    {
        Ok(limit) => limit,
        Err(error) => {
            log::error!("Could not apply Atlas memory limit: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = game::run() {
        log::error!("Atlas: {error}");
        if !help {
            crash::report_error(&home, side, &*error);
        }
        // Started from Explorer, the console window closes with the game:
        // keep the message on screen until it is read.
        #[cfg(windows)]
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            eprintln!("Press Enter to close.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(1);
    }
    logging::flush();
}
