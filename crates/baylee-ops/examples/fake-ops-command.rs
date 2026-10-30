//! Native command stand-ins used only by the operations integration tests.
use serde_json::{Value, json};
use std::{fs, io::Write as _, path::Path};

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let name = Path::new(&arguments[0])
        .file_name()
        .unwrap()
        .to_string_lossy();
    if !matches!(name.as_ref(), "date" | "curl" | "systemctl" | "logger") {
        // Cargo may run the example directly as an empty test target.
        return;
    }
    let calls = std::env::var_os("OPS_FAKE_CALLS").expect("private test call log");
    let mut log = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(calls)
        .unwrap();
    writeln!(log, "{name} {}", arguments[1..].join(" ")).unwrap();
    match name.as_ref() {
        "date" => {
            assert_eq!(&arguments[1..], ["+%H"]);
            println!("{:02}", state()["hour"].as_u64().unwrap());
        }
        "curl" => health(),
        "systemctl" | "logger" => {}
        _ => unreachable!(),
    }
}

fn state() -> Value {
    serde_json::from_slice(&fs::read(std::env::var_os("OPS_FAKE_STATE").unwrap()).unwrap()).unwrap()
}

fn health() {
    if let Ok(body) = std::env::var("OPS_FAKE_HEALTH") {
        println!("{body}");
        return;
    }
    let state = state();
    let Some(initial) = state["health"].as_array() else {
        std::process::exit(7);
    };
    let runtime = std::env::var_os("BAYLEE_REBOOT_RUN").unwrap();
    let held = Path::new(&runtime).join("admission-hold").exists();
    let admission = if held {
        state["admission"].as_str().unwrap()
    } else {
        "open"
    };
    if admission == "down" {
        std::process::exit(7);
    }
    let counts = if held {
        state["health_after_hold"].as_array().unwrap_or(initial)
    } else {
        initial
    };
    let running = counts[0].as_u64().unwrap();
    println!(
        "{}",
        json!({
            "admission": admission,
            "games": {"running": running + 5, "local_running": running, "waiting": counts[1]}
        })
    );
}
