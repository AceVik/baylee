//! The desktop entry point. The copy in a package is never replaced by an automatic
//! update: it hands each start to the selected generation's copy of this same program
//! (`baylee_update::bootstrap`).
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    let os = if cfg!(target_os = "windows") {
        baylee_update::plan::Os::Windows
    } else if cfg!(target_os = "macos") {
        baylee_update::plan::Os::MacOs
    } else {
        baylee_update::plan::Os::Linux
    };
    let result = std::env::current_exe().and_then(|exe| {
        baylee_update::bootstrap::start(&exe, os, std::env::args_os().skip(1).collect())
    });
    match result {
        Ok(status) if status.success() => std::process::ExitCode::SUCCESS,
        Ok(_) => std::process::ExitCode::FAILURE,
        Err(err) => {
            eprintln!("Baylee could not start (another instance may be running): {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
