//! `explain` and `pool-dump`.

use crate::{BTreeMap, Path, fs, refusal_cause, scriptgen, scripts_root, scryfall, tokengen};

pub(crate) fn pool_dump(out: &Path) -> anyhow::Result<()> {
    use std::fmt::Write as _;
    let mut text = String::new();
    for def in baylee_cards::all() {
        let _ = writeln!(text, "{def:#?}");
    }
    fs::write(out, &text)?;
    println!(
        "pool dump: {} cards -> {}",
        baylee_cards::count(),
        out.display()
    );
    Ok(())
}

pub(crate) fn explain(
    root: &Path,
    name: &str,
    scripts_dir: &Path,
    cache: &Path,
) -> anyhow::Result<()> {
    let cache = root.join(cache);
    let agent = ureq::Agent::new_with_defaults();
    let card = scryfall::fetch_named(name, &agent, &cache)?;
    println!(
        "== Scryfall ==\n{} — {} — {}\n{}\n",
        card.name,
        card.mana_cost.as_deref().unwrap_or(""),
        card.type_line.as_deref().unwrap_or(""),
        card.oracle_text.as_deref().unwrap_or("")
    );
    let index_path = root.join("data/script-index.json");
    if index_path.exists() {
        let index: BTreeMap<String, String> =
            serde_json::from_str(&fs::read_to_string(&index_path)?)?;
        if let Some(rel) = index.get(name) {
            let script = scripts_root(root, scripts_dir).join(rel);
            println!("== card-script reference ({}) ==", script.display());
            let text = fs::read_to_string(&script).unwrap_or_default();
            println!("{text}");
            // What the transcoder makes of it, which is the question a
            // person opening this tool on a stub is actually asking. It was
            // answerable only by a corpus-wide `transcode-report` run,
            // whose ranking is about the corpus and not about this card.
            let cats = scryfall::fetch_subtype_catalogs(&agent, &cache)?;
            let parsed = scriptgen::parse(&text);
            println!("== transcoder ==");
            let tokens = tokengen::TokenLookup::beside(&scripts_root(root, scripts_dir))?;
            if scriptgen::transcode(&parsed, &cats, tokens.as_ref()).is_some() {
                println!("read in full");
            } else {
                println!(
                    "refused: {}",
                    refusal_cause(&parsed, &cats, tokens.as_ref())
                );
            }
        } else {
            println!("card-script reference: no script found for {name:?}");
        }
    } else {
        println!("note: data/script-index.json missing; run `cargo xtask codegen`");
    }
    Ok(())
}
