//! Offline preparation artifact; intentionally not a publishable execution contract.
#![forbid(unsafe_code)]
use baylee_cards_pool as _;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = baylee_tower_content::expeditions()?;
    let mut output = std::io::stdout().lock();
    if std::env::args().nth(1).as_deref() == Some("--text") {
        for starter in &content.starters {
            let leader = starter
                .formation
                .as_ref()
                .ok_or("missing commander")?
                .commander;
            writeln!(
                output,
                "\n# {}",
                baylee_cards::by_index(leader).ok_or("unknown card")?.name()
            )?;
            for row in &starter.deck {
                writeln!(
                    output,
                    "{} {}",
                    row.copies,
                    baylee_cards::by_index(row.card)
                        .ok_or("unknown card")?
                        .name()
                )?;
            }
        }
        return Ok(());
    }
    serde_json::to_writer_pretty(&mut output, &content)?;
    writeln!(output)?;
    Ok(())
}
