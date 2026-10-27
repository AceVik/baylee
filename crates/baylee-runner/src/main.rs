//! The native macOS GitHub Actions runner executable.

fn main() {
    if let Err(error) = baylee_runner::main_entry() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
