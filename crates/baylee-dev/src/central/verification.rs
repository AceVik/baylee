//! Run the existing mandatory gates against synthetic PostgreSQL, with owned cleanup.
use super::{Duration, Owned, Result, cargo, ensure, with_postgres};

pub(crate) fn gate(features: bool, failure: bool) -> Result<i32> {
    with_postgres(
        "baylee-challenge-gate",
        "2g",
        2_147_483_648,
        failure,
        |pg| {
            let mut command = cargo();
            command.args(["dev", "gate"]);
            if features {
                command.arg("features");
            }
            command
                .env_remove("POSTGRES_PASSWORD")
                .env_remove("PGPASSWORD")
                .env("DATABASE_URL", pg.admin_url());
            println!(
                "Running the required {} gate with isolated PostgreSQL {}.",
                if features { "features" } else { "full" },
                pg.version()?
            );
            let done = Owned::spawn(&mut command)?.wait(Duration::from_secs(21_600), None)?;
            print!("{}{}", done.stdout, done.stderr);
            ensure!(done.status.success(), "required gate failed");
            Ok(())
        },
    )
}
