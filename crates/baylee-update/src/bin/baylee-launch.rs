//! Permanent desktop entry point. Never replaced by an automatic update.
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
        let install = baylee_update::apply::Install::around_launcher(&exe, os)
            .map_err(std::io::Error::other)?;
        baylee_update::launch::run(&install, std::env::args_os().skip(1))
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
