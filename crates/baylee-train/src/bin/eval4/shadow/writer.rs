//! Streaming publication, bound to original executed record inputs.
use super::{Counts, sha};
use anyhow::{Context, ensure};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub(super) struct LabelWriter {
    draft: PathBuf,
    final_path: PathBuf,
    file: BufWriter<File>,
    /// Where a watching model's own log of this game goes.
    transcripts: PathBuf,
}

impl LabelWriter {
    pub(super) fn new(dir: &Path, game: &str, header: &Value) -> anyhow::Result<Self> {
        ensure!(
            !game.is_empty() && game.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
            "Invalid shadow game key"
        );
        let directory = dir.join("shadow");
        std::fs::create_dir_all(&directory)?;
        let final_path = directory.join(format!("{game}.jsonl"));
        ensure!(!final_path.exists(), "Preserving existing shadow labels");
        let draft = directory.join(format!("{game}.draft"));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&draft)?;
        let mut result = Self {
            draft,
            final_path,
            file: BufWriter::new(file),
            transcripts: directory.join("transcripts").join(game),
        };
        result.write(header)?;
        Ok(result)
    }

    /// The directory a watching model's own log of this game goes to.
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    pub(super) fn transcripts(&self) -> PathBuf {
        self.transcripts.clone()
    }

    pub(super) fn write(&mut self, value: &Value) -> anyhow::Result<()> {
        serde_json::to_writer(&mut self.file, value)?;
        writeln!(self.file)?;
        Ok(())
    }

    /// Publishes the labels bound to `record`; a watching model's seat
    /// notes (`transcript`) go beside them as `<game>.seat.jsonl`, each of
    /// its labels bound to the mind's own answer there.
    pub(super) fn finish(
        mut self,
        record: &[u8],
        capped: bool,
        counts: &Counts,
        transcript: Option<&[u8]>,
    ) -> anyhow::Result<()> {
        self.file.flush()?;
        if capped {
            // Drafts are explicitly NOT consumable labels, even if all their
            // individual decisions mapped. No terminal outcome exists.
            self.write(
                &json!({"kind":"incomplete", "reason":"answer_cap", "counts":counts,
                "label_use":"bc_only", "usable":false}),
            )?;
            self.file.flush()?;
            return Ok(());
        }
        ensure!(
            record
                .split(|b| *b == b'\n')
                .rfind(|line| !line.is_empty())
                .is_some_and(
                    |line| serde_json::from_slice::<Value>(line).is_ok_and(|v| v["kind"] == "end")
                ),
            "Shadow publication requires an End record"
        );
        let publish = self.final_path.with_extension("publishing");
        let mut output = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&publish)?,
        );
        let answers = transcript
            .map(baylee_train::convert4::shadow_bc::mind_answers)
            .transpose()?;
        let mut source = None;
        let mut last_offset = None;
        for line in BufReader::new(File::open(&self.draft)?).lines() {
            let mut row: Value = serde_json::from_str(&line?)?;
            ensure!(
                matches!(row["kind"].as_str(), Some("header" | "label")),
                "Unknown shadow row"
            );
            if row["kind"] == "header" {
                let teacher = row["contract"]["teacher"].as_str().context("Teacher")?;
                source = Some(if teacher.starts_with("llm:") {
                    "llm"
                } else {
                    "house"
                });
                ensure!(
                    (source == Some("llm")) == answers.is_some(),
                    "A watching model's labels need its transcript, and only those"
                );
            }
            if row["kind"] == "label" {
                ensure!(
                    row["label_use"] == "bc_only"
                        && source.is_some_and(|source| row["teacher_source"] == source)
                        && row["teacher_has_logprob"] == false,
                    "Invalid shadow label provenance"
                );
                if let Some(answers) = &answers {
                    let question = row["teacher_question"].as_u64().context("Question")?;
                    ensure!(
                        answers.get(&question) == Some(&row["teacher_action"]),
                        "Shadow label is not the watching model's answer"
                    );
                }
                let offset = row["record_byte_offset"].as_u64().context("Label offset")?;
                ensure!(
                    last_offset.is_none_or(|last| offset > last),
                    "Nonchronological label offsets"
                );
                last_offset = Some(offset);
                bind_input(&mut row, record)?;
            }
            serde_json::to_writer(&mut output, &row)?;
            writeln!(output)?;
        }
        let mut completed = json!({"kind":"completed", "schema":1,
            "record_sha256":sha(record), "record_bytes":record.len(), "counts":counts,
            "label_use":"bc_only", "ppo_certification":false});
        if let Some(transcript) = transcript {
            let path = self.final_path.with_extension("seat.jsonl");
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(transcript)?;
            file.sync_all()?;
            completed["transcript_sha256"] = json!(sha(transcript));
        }
        serde_json::to_writer(&mut output, &completed)?;
        writeln!(output)?;
        output.flush()?;
        output.get_ref().sync_all()?;
        // Hard-link is atomic and refuses overwrite, unlike rename on Unix.
        std::fs::hard_link(&publish, &self.final_path)?;
        std::fs::remove_file(publish)?;
        std::fs::remove_file(self.draft)?;
        Ok(())
    }
}

fn bind_input(row: &mut Value, record: &[u8]) -> anyhow::Result<()> {
    let offset = usize::try_from(row["record_byte_offset"].as_u64().context("Label offset")?)?;
    ensure!(
        offset > 0 && offset < record.len() && record[offset - 1] == b'\n',
        "Invalid record boundary"
    );
    let mut input = None;
    for line in record[offset..]
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: Value = serde_json::from_slice(line)?;
        match value["kind"].as_str() {
            // A first socket answer can record its mind declaration first.
            Some("declared_mind") => {}
            Some("input") => {
                input = Some(value);
                break;
            }
            _ => anyhow::bail!("Unexpected line before bound executed input"),
        }
    }
    let input = input.context("Missing bound executed input")?;
    ensure!(
        input["seat"] == row["seat"]
            && input["action"] == row["executed_action"]
            && input["by"] == "seat",
        "Shadow label does not match executed record input"
    );
    row["record_input_n"] = input["n"].clone();
    row["record_input_hash"] = input["hash"].clone();
    Ok(())
}
