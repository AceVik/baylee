//! Offline content preparation, with no credentials, database or network access.
#![forbid(unsafe_code)]
use baylee_cards_pool as _;
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let content = baylee_tower_content::runic()?;
    let value = match args.as_slice() {
        [mode] if mode == "--check" => serde_json::json!({
            "generation": baylee_tower::v2::GENERATOR,
            "starters": content.starters.len(),
            "tested_cards": content.catalog.cards.len(),
            "fingerprint": baylee_tower_content::fingerprint(&content)?,
            "calibrated": baylee_tower_content::fingerprint(&content)? == baylee_tower_content::CALIBRATED,
        }),
        [mode] if mode == "--catalog" => serde_json::to_value(content)?,
        [mode] if mode == "--publication" => {
            let mut input = Vec::new();
            std::io::stdin().take(32_769).read_to_end(&mut input)?;
            if input.len() > 32_768 {
                return Err("executor identity exceeds 32 KiB".into());
            }
            let identity: baylee_protocol::execution::Identity = serde_json::from_slice(&input)?;
            let cohort = baylee_tower::v2::package::Cohort::for_generation(
                identity,
                &content.catalog,
                baylee_tower::v2::GENERATOR,
            )?;
            serde_json::json!({"cohort": cohort, "catalog": content.catalog, "starters": content.starters})
        }
        _ => {
            return Err(
                "use --check, --catalog, or --publication (exact tower-6 identity on stdin)".into(),
            );
        }
    };
    let mut output = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, &value)?;
    writeln!(output)?;
    Ok(())
}
