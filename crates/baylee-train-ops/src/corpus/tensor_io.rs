//! Lock-held native publication around a tensor worker's explicit success handshake.
use crate::files;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
struct Publication {
    out: PathBuf,
    temporary: Option<PathBuf>,
    directory: bool,
    cache: bool,
    cache_options: Value,
    _lock: Option<File>,
}
impl Drop for Publication {
    fn drop(&mut self) {
        if let Some(path) = self.temporary.take() {
            let _ = if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
        }
    }
}
fn output(value: &Value) -> Result<PathBuf> {
    let out = Path::new(value["out"].as_str().context("missing tensor output")?);
    let out = std::path::absolute(out)?;
    let parent = out.parent().context("tensor output needs parent")?;
    std::fs::create_dir_all(parent)?;
    Ok(parent
        .canonicalize()?
        .join(out.file_name().context("missing output filename")?))
}
impl Publication {
    fn prepare(value: &Value) -> Result<(Self, Value)> {
        let out = output(value)?;
        let parent = out.parent().context("missing output parent")?;
        let name = out
            .file_name()
            .context("missing output name")?
            .to_string_lossy();
        let directory = value["kind"] != "file";
        let cache = value["kind"] == "cache";
        if cache {
            ensure!(
                out.file_name().and_then(|n| n.to_str())
                    == Some(super::bc_metadata::spaced_digest(&value["settings"])?.as_str()),
                "Calibration destination does not match its settings key"
            );
        }
        ensure!(
            matches!(value["kind"].as_str(), Some("file" | "directory" | "cache")),
            "unsupported tensor publication kind"
        );
        let lock = if directory {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(parent.join(format!(".{name}.tensor-publish.lock")))?;
            file.lock()?;
            Some(file)
        } else {
            None
        };
        if directory && out.symlink_metadata().is_ok() {
            ensure!(cache, "Preserving existing output; use a new directory");
            let receipt = super::calibration_io::admit(
                &out,
                value["width"].as_u64().context("missing feature width")?,
                value.get("settings"),
                value["max_samples"].as_u64(),
            )?;
            return Ok((
                Self {
                    out,
                    temporary: None,
                    directory,
                    cache,
                    cache_options: value.clone(),
                    _lock: lock,
                },
                json!({"reused":true,"cache":receipt}),
            ));
        }
        let temporary = parent.join(format!("{name}.partial-{}", uuid::Uuid::now_v7()));
        if directory {
            std::fs::create_dir(&temporary)?;
        } else {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
        }
        let receipt = json!({"temporary":temporary,"reused":false});
        Ok((
            Self {
                out,
                temporary: Some(temporary),
                directory,
                cache,
                cache_options: value.clone(),
                _lock: lock,
            },
            receipt,
        ))
    }
    fn write(&self, length: u64, input: &mut impl std::io::Read) -> Result<Value> {
        ensure!(
            !self.directory,
            "raw writes require a tensor file reservation"
        );
        let path = self
            .temporary
            .as_ref()
            .context("no active tensor reservation")?;
        let mut file = OpenOptions::new().append(true).open(path)?;
        let written = std::io::copy(&mut std::io::Read::take(input, length), &mut file)?;
        ensure!(
            written == length,
            "Incomplete tensor transport block; publication aborted"
        );
        Ok(json!({"written":written}))
    }
    fn commit(&mut self, value: &Value) -> Result<Value> {
        let temporary = self
            .temporary
            .as_ref()
            .context("no active tensor reservation")?;
        let result = if self.directory {
            ensure!(
                !self.out.exists() && !self.out.is_symlink(),
                "Preserving existing output; use a new directory"
            );
            let name = if self.cache {
                "complete.json"
            } else {
                "report.json"
            };
            let payload = temporary.join(if self.cache { "records.bin" } else { "net.pt" });
            ensure!(
                payload.symlink_metadata()?.file_type().is_file(),
                "tensor payload must be a regular file"
            );
            File::open(&payload)?.sync_all()?;
            files::atomic_json(&temporary.join(name), &value["metadata"])?;
            if self.cache {
                super::calibration_io::admit(
                    temporary,
                    self.cache_options["width"]
                        .as_u64()
                        .context("missing feature width")?,
                    self.cache_options.get("settings"),
                    self.cache_options["max_samples"].as_u64(),
                )?;
            } else {
                ensure!(
                    payload.metadata()?.len() > 0,
                    "empty serialized tensor checkpoint"
                );
            }
            File::open(temporary)?.sync_all()?;
            json!({"out":self.out})
        } else {
            ensure!(
                temporary.symlink_metadata()?.file_type().is_file(),
                "tensor payload must be a regular file"
            );
            File::open(temporary)?.sync_all()?;
            ensure!(
                temporary.metadata()?.len() > 0,
                "empty serialized tensor payload"
            );
            json!({"out":self.out,"sha256":files::sha256(temporary)?})
        };
        std::fs::rename(temporary, &self.out)?;
        File::open(self.out.parent().context("missing publication parent")?)?.sync_all()?;
        self.temporary = None;
        Ok(result)
    }
}
/// Serve a bounded tensor publication session; EOF/error aborts unpublished output.
///
/// # Errors
/// Returns transport errors; operation failures are explicit JSON error responses.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> Result<()> {
    let mut publication = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        let request: Value = serde_json::from_str(&line)?;
        let op = request["op"].as_str().context("missing tensor operation")?;
        let result = (|| -> Result<Value> {
            match op {
                "prepare" => {
                    ensure!(publication.is_none(), "tensor session already prepared");
                    let (state, receipt) = Publication::prepare(&request)?;
                    publication = Some(state);
                    Ok(receipt)
                }
                "write" => publication
                    .as_ref()
                    .context("tensor session is not prepared")?
                    .write(
                        request["length"]
                            .as_u64()
                            .context("missing tensor transport length")?,
                        &mut input,
                    ),
                "commit" => publication
                    .as_mut()
                    .context("tensor session is not prepared")?
                    .commit(&request),
                "abort" => {
                    publication = None;
                    Ok(json!({"aborted":true}))
                }
                _ => bail!("unsupported tensor operation"),
            }
        })();
        let failed = result.is_err();
        let response = match result {
            Ok(value) => json!({"ok":true,"value":value}),
            Err(error) => {
                json!({"ok":false,"exists":error.to_string().starts_with("Preserving existing output"),"error":format!("{error:#}")})
            }
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
        if failed || matches!(op, "commit" | "abort") {
            break;
        }
    }
    Ok(())
}
#[cfg(test)]
#[path = "tensor_io_tests.rs"]
mod tests;
