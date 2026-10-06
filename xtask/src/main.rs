//! xtask — baylee development tasks (codegen, card explanation, …).

mod adopt;
mod batch;
mod card_batch;
mod check_card;
mod check_costs;
mod check_text;
mod cli;
mod codegen;
mod corpus;
mod cr_check;
mod cross_read;
mod dev_table;
mod explain;
mod files;
mod hooks;
mod ledger_cmd;
mod mechanics;
mod oracle_lines;
mod precons;
mod printings;
mod tables;
mod transcode;
mod update_key;
mod validate;
mod verify;

use adopt::{adopt, adopt_stub_card, refresh_oracle, set_header_line};
use batch::{batch, collect_scripts, coverage_set, reach_list, refusal_cause, stub_names};
#[cfg(test)]
use batch::{drift_from_status, script_for};
use baylee_cards_codegen::{
    acceptance, cardindex, catalog, landgen, layout, ledger, lines, names, scriptgen, scripts,
    scryfall, stubgen, tokengen, tokenledger,
};
use card_batch::{card_batch, land_report};
#[cfg(test)]
use check_card::{PrintedBound, printed_enters_tapped_bound, printed_subtypes};
use check_card::{check_def_against_the_printing, check_header_matches_code};
#[cfg(test)]
use check_costs::{PrintedMana, printed_mana_offered};
use check_costs::{
    check_activation_cost_matches_the_printing, check_code_matches_the_printing,
    check_mana_matches_the_printing, code_costs,
};
use check_text::{
    KEYWORD_WORDS, check_optional_clauses_are_offered, check_oracle_matches_the_printing,
    check_player_targets_match_the_printing, check_scope_matches_the_text,
    check_search_tapped_matches_text, check_set_line_matches_the_printing,
    check_target_counts_match_the_printing, face_texts, mentions_word, printed_text,
    strip_reminders,
};
use clap::{Parser, Subcommand};
use cli::{Cli, Cmd};
#[cfg(test)]
use codegen::refuse_twin_names;
use codegen::{Pool, codegen, front_face_slug};
#[cfg(test)]
use corpus::one_row_per_token;
use corpus::{scripts_root, token_ledger};
use cross_read::cross_read;
use dev_table::{TableSpec, dev_table};
#[cfg(test)]
use dev_table::{arrange_room, bridge_mind, table_deck, table_deck_names};
use explain::{explain, pool_dump};
use files::{card_files, knob, quoted_value, relative, write_or_check, write_verbatim};
#[cfg(test)]
use files::{format_rust, format_rust_many};
use ledger_cmd::ledger_cmd;
use oracle_lines::{render_ability_lines, render_oracle};
#[cfg(test)]
use printings::header_scryfall_id;
use printings::{
    Pinned, cached_printing, pinned_printing, refresh_payload_cache, scryfall_cache, unplayable_ids,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use tables::{
    ability_lines, check_no_name_is_claimed_twice, render_name_table, render_sides_table,
    render_token_ledger,
};
use transcode::{Narrow, pool_additions, transcode_report};
use validate::{PrintingTally, deck_check, validate};

#[allow(clippy::too_many_lines)] // one flat table: a subcommand, its function
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in <workspace>/xtask")
        .to_path_buf();
    match cli.cmd {
        Cmd::Codegen {
            adopt_stub,
            check,
            tables,
            scripts,
            cache,
        } => match adopt_stub {
            Some(name) => adopt_stub_card(&root, &name),
            None => codegen(&root, check, tables, &scripts, &cache),
        },
        Cmd::Ledger {
            corpus,
            check,
            reseed,
        } => ledger_cmd(&root, &corpus, check, reseed),
        Cmd::AbilityLines => ability_lines(&root),
        Cmd::DeckCheck { file, verbose } => deck_check(&root, &file, verbose),
        Cmd::Verify(args) => verify::verify(&root, &args),
        Cmd::DecksImport { archive, refresh } => {
            precons::import(&root, archive.as_deref(), refresh)
        }
        Cmd::DecksStatus { check } => precons::status(&root, check),
        Cmd::PoolDump { out } => pool_dump(&out),
        Cmd::TranscodeReport {
            scripts,
            samples,
            stubs,
            names,
            reason,
            causes,
        } => transcode_report(
            &root,
            &scripts,
            samples,
            &Narrow {
                stubs,
                names: names.as_deref(),
            },
            reason.as_deref(),
            causes,
        ),
        Cmd::Batch {
            count,
            dry_run,
            scripts,
            cache,
        } => batch(&root, &scripts, &cache, count, dry_run),
        Cmd::ReachList {
            scripts,
            out,
            count,
            cache,
        } => reach_list(&root, &scripts, out.as_deref(), count, &cache),
        Cmd::CrossRead { scripts, samples } => cross_read(&root, &scripts, samples),
        Cmd::CrCheck { rules, all } => cr_check::run(&root, &rules, all),
        Cmd::LandReport {
            samples,
            worklist,
            tail,
            cache,
        } => land_report(&root, &cache, samples, worklist.as_deref(), tail),
        Cmd::CoverageSet {
            count,
            max_new,
            scripts,
        } => coverage_set(&root, &scripts, count, max_new),
        Cmd::Explain {
            name,
            scripts,
            cache,
        } => explain(&root, &name, &scripts, &cache),
        Cmd::CardBatch {
            cards,
            out,
            scripts,
            cache,
        } => card_batch(&root, cards.as_deref(), &out, &scripts, &cache),
        Cmd::ScryfallCache { cache, refetch } => scryfall_cache(&root, &cache, refetch),
        Cmd::Validate => validate(&root),
        Cmd::Adopt { name } => adopt(&root, &name),
        Cmd::RefreshOracle { dry_run } => refresh_oracle(&root, dry_run),
        Cmd::DevTable {
            gateway,
            seats,
            ai,
            deck,
            teams,
            play,
            bridges,
        } => dev_table(
            &root,
            &gateway,
            &TableSpec {
                seats,
                ai: &ai,
                deck: &deck,
                teams: &teams,
                bridges: &bridges,
            },
            play,
        ),
        Cmd::UpdateKey { out } => update_key::run(&out.map_or_else(update_key::default_path, Ok)?),
    }
}

#[cfg(test)]
mod tests;
