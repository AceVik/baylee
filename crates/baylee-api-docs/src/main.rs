//! Writes or checks the API documents in `docs/api/` from the servers'
//! sources: `write` (the default) rewrites the documents that changed, and
//! `check` writes nothing and fails on a stale or missing one (the gate's
//! `api` step and CI's `validate` job). `serve [--port N]` shows them in
//! Swagger UI on this machine ([`serve`]). Writing or checking, either way
//! every route a service's routers register must be in one of its documents,
//! and nothing else ([`routes`]). The repository root is the working directory
//! (`cargo dev api` sets it).

mod routes;
mod serve;

use std::collections::BTreeSet;
use std::process::ExitCode;

use oas_forge::{Generator, OutputKind, Package};

/// One generator run: the sources it reads (a service and the types its
/// answers name), the `.proto` files, and the documents it writes by scope.
/// The gateway and the feedback service are separate runs because each has
/// its own `ErrorBody`, `Health` and others of one name.
struct Run {
    inputs: &'static [&'static str],
    protos: &'static [&'static str],
    outputs: &'static [(&'static str, Kind, &'static str)],
}

const RUNS: &[Run] = &[
    Run {
        inputs: &[
            "crates/baylee-gateway/src",
            "crates/baylee-protocol/src",
            "crates/baylee-core/src",
            "crates/baylee-cards/src/pool.rs",
            "crates/baylee-catalog/src",
            "crates/baylee-cardtext/src",
            "crates/baylee-db/src/console.rs",
        ],
        protos: &["crates/baylee-protocol/proto/baylee/v1/transport.proto"],
        outputs: &[
            ("public", Kind::OpenApi, "docs/api/gateway.openapi.yaml"),
            (
                "admin",
                Kind::OpenApi,
                "docs/api/gateway-admin.openapi.yaml",
            ),
            ("public", Kind::AsyncApi, "docs/api/gateway.asyncapi.yaml"),
        ],
    },
    Run {
        inputs: &["crates/baylee-feedback/src"],
        protos: &[],
        outputs: &[("feedback", Kind::OpenApi, "docs/api/feedback.openapi.yaml")],
    },
];

#[derive(Clone, Copy)]
enum Kind {
    OpenApi,
    AsyncApi,
}

/// One service: the sources whose routers it registers, the documents that
/// together describe them, and the paths none of them does (a catch-all).
struct Service {
    routers: &'static str,
    documents: &'static [&'static str],
    undocumented: &'static [&'static str],
}

const SERVICES: &[Service] = &[
    Service {
        routers: "crates/baylee-gateway/src",
        documents: &[
            "docs/api/gateway.openapi.yaml",
            "docs/api/gateway-admin.openapi.yaml",
            "docs/api/gateway.asyncapi.yaml",
        ],
        undocumented: &[],
    },
    Service {
        routers: "crates/baylee-feedback/src",
        documents: &["docs/api/feedback.openapi.yaml"],
        undocumented: &["/ui/api/{*rest}"],
    },
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = match args.first().map(String::as_str) {
        None | Some("write") => Task::Write,
        Some("check") => Task::Check,
        Some("serve") => {
            let port = match args.get(1..).unwrap_or_default() {
                [] => serve::PORT,
                [flag, port] if flag == "--port" => {
                    let Ok(port) = port.parse() else {
                        eprintln!("api-docs: `{port}` is not a port");
                        return ExitCode::from(2);
                    };
                    port
                }
                _ => {
                    eprintln!("api-docs: serve [--port N]");
                    return ExitCode::from(2);
                }
            };
            return match serve::run(port) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("api-docs: {error}");
                    ExitCode::FAILURE
                }
            };
        }
        Some(other) => {
            eprintln!("api-docs: unknown task `{other}` (write, check, serve)");
            return ExitCode::from(2);
        }
    };
    match run(task) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("api-docs: {error}");
            ExitCode::FAILURE
        }
    }
}

/// What to do with each document.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Task {
    Write,
    Check,
}

/// Builds every document and writes or checks them; `false` when one has
/// errors, a service's documents miss a route or hold one too many, or,
/// checking, one is stale.
fn run(task: Task) -> Result<bool, Box<dyn std::error::Error>> {
    // `{{CARGO_PKG_VERSION}}`: every crate here shares the workspace's.
    let manifest = "crates/baylee-gateway/Cargo.toml";
    let mut fresh = true;
    for run in RUNS {
        let package = Package::from_manifest(manifest)?
            .ok_or_else(|| format!("{manifest} names no package"))?;
        let mut generator = Generator::new().package(package);
        for input in run.inputs {
            generator = generator.input(*input);
        }
        for proto in run.protos {
            generator = generator.include(*proto);
        }
        for (scope, kind, path) in run.outputs {
            let kind = match kind {
                Kind::OpenApi => OutputKind::Full,
                Kind::AsyncApi => OutputKind::AsyncApi,
            };
            generator = generator.output_scoped(*scope, kind, *path);
        }
        let spec = generator.build()?;
        for diagnostic in spec.diagnostics() {
            eprintln!("{diagnostic}");
        }
        if spec.diagnostics().has_errors() {
            return Ok(false);
        }
        if task == Task::Write {
            spec.write()?;
        } else {
            for path in spec.check()? {
                eprintln!("api-docs: {} is stale; run `cargo dev api`", path.display());
                fresh = false;
            }
        }
    }
    let mut complete = true;
    for service in SERVICES {
        complete &= every_route_documented(service)?;
    }
    Ok(complete && fresh)
}

/// Whether a service's documents together hold exactly the operations its
/// routers register (a socket as `GET` on its channel's address), less the
/// paths none documents; each difference is printed.
fn every_route_documented(service: &Service) -> Result<bool, Box<dyn std::error::Error>> {
    let registered: BTreeSet<routes::Operation> = routes::registered(service.routers.as_ref())?
        .into_iter()
        .filter(|(_, path)| !service.undocumented.contains(&path.as_str()))
        .collect();
    let mut documented = BTreeSet::new();
    for document in service.documents {
        let written = std::fs::read_to_string(document)?;
        documented.extend(routes::documented(&written));
        documented.extend(routes::channels(&written));
    }
    // A reader that misread would find nothing on one side and pass.
    if registered.len() < 10 || documented.len() < 10 {
        return Err(format!(
            "{}: {} routes registered and {} documented; a reader misread",
            service.routers,
            registered.len(),
            documented.len()
        )
        .into());
    }
    for (method, path) in registered.difference(&documented) {
        eprintln!(
            "api-docs: {}: {} {path} is registered and in no document",
            service.routers,
            method.to_uppercase()
        );
    }
    for (method, path) in documented.difference(&registered) {
        eprintln!(
            "api-docs: {}: {} {path} is documented and not registered",
            service.routers,
            method.to_uppercase()
        );
    }
    Ok(registered == documented)
}
