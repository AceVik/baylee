//! Explicit finalized-ledger adapter; incomplete index rows never acquire targets.
use super::*;
#[cfg(target_os = "linux")]
use baylee_train_ops::research_attempts::{Attempt, verify_finalized};

pub(super) struct Accounting {
    pub ledger: Value,
    planned: BTreeMap<u64, Value>,
}

impl Run {
    #[cfg(target_os = "linux")]
    pub fn load_finalized(path: &Path) -> anyhow::Result<Self> {
        let root = path.canonicalize()?;
        let ledger = verify_finalized(&root)?;
        let mut seals: BTreeMap<PathBuf, String> =
            serde_json::from_value(ledger["source_files"].clone())?;
        for name in ["attempts.final.json", "attempts.complete.json"] {
            let path = bound(&root, &root.join(name))?;
            seals.insert(path.clone(), files::sha256(&path)?);
        }
        let metadata = files::read_json(&root.join("run.json"))?;
        let encoder = validate_metadata(&metadata)?;
        let source_sha = seals[&root.join("run.json")].clone();
        let index_sha = seals[&root.join("games.jsonl")].clone();
        let plan = files::read_json(&root.join("attempts.plan.json"))?;
        let attempts: Vec<Attempt> = serde_json::from_value(plan["attempts"].clone())?;
        let deck_keys: Vec<String> = serde_json::from_value(metadata["accepted_deck_keys"].clone())
            .context("finalized research source requires original accepted deck keys")?;
        ensure!(
            deck_keys.len() >= 2,
            "research source has no original deck schedule"
        );
        let mut planned = BTreeMap::new();
        for attempt in attempts {
            let first_deck = usize::try_from(attempt.deal / 2)?
                .checked_mul(2)
                .context("deck schedule overflow")?;
            ensure!(
                attempt.focal_seat == usize::try_from(attempt.deal % 2)?
                    && attempt.deck_keys.len() == 2
                    && attempt
                        .deck_keys
                        .iter()
                        .enumerate()
                        .all(|(seat, key)| !key.is_empty()
                            && *key == deck_keys[(first_deck + seat) % deck_keys.len()]),
                "research plan role/deck schedule differs from sealed source"
            );
            planned.insert(attempt.deal, serde_json::to_value(attempt)?);
        }
        let mut games = Vec::new();
        let mut keys = BTreeSet::new();
        for row in ledger["attempts"]
            .as_array()
            .context("finalized attempts")?
        {
            if row["status"] != "recorded" {
                continue;
            }
            let path = Path::new(
                row["game_receipt"]
                    .as_str()
                    .context("finalized game receipt")?,
            );
            let game = files::read_json(path)?;
            let key = game["game"].as_str().context("research source game key")?;
            ensure!(
                !key.is_empty() && keys.insert(key.to_owned()),
                "empty or duplicate research source key"
            );
            games.push(Game::load(&root, &metadata, &source_sha, game, &mut seals)?);
        }
        games.sort_by_key(|game| game.row["i"].as_u64());
        let run = Self {
            root,
            metadata,
            encoder,
            seals,
            source_sha,
            index_sha,
            games,
            accounting: Some(Accounting { ledger, planned }),
        };
        run.recheck()?;
        Ok(run)
    }

    #[cfg(not(target_os = "linux"))]
    pub fn load_finalized(_: &Path) -> anyhow::Result<Self> {
        anyhow::bail!("finalized research process accounting requires Linux");
    }

    pub fn attempts(&self) -> anyhow::Result<Vec<SourceAttempt<'_>>> {
        let Some(accounting) = &self.accounting else {
            return Ok(self
                .games
                .iter()
                .map(|game| SourceAttempt {
                    game: Some(game),
                    accounting: Value::Null,
                })
                .collect());
        };
        let games: BTreeMap<_, _> = self
            .games
            .iter()
            .map(|game| (game.row["i"].as_u64().expect("verified game index"), game))
            .collect();
        let mut attempts = Vec::new();
        for row in accounting.ledger["attempts"]
            .as_array()
            .context("finalized attempts")?
        {
            let deal = row["deal"].as_u64().context("finalized deal")?;
            let game = games.get(&deal).copied();
            ensure!(
                (row["status"] == "recorded") == game.is_some(),
                "recorded attempt lost its verified source receipt"
            );
            attempts.push(SourceAttempt { game, accounting: json!({"source_index":deal,
                "source_attempt_status":row["status"],"assigned":row["assigned"],
                "planned_attempt":accounting.planned.get(&deal).context("missing sealed plan attempt")?,
                "source_attempt_ledger_entry":row}) });
        }
        attempts.sort_by_key(|attempt| attempt.accounting["source_index"].as_u64());
        Ok(attempts)
    }
}
