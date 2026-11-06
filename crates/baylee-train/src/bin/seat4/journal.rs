//! What this seat saw and answered, for a reader of a game: the loss analysis of
//! the first ladder could not see the net's hand. Local and for numbers only:
//! it holds this seat's own view and nothing it was not shown, and it is never
//! a game record or training data.
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::Path;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::Value;

/// How many rows a stopped game may lose.
const ROWS_PER_FLUSH: u32 = 16;

pub struct Journal {
    file: GzEncoder<BufWriter<File>>,
    rows: u32,
}

impl Journal {
    pub fn create(path: &Path) -> anyhow::Result<Self> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        Ok(Self {
            file: GzEncoder::new(BufWriter::new(File::create(path)?), Compression::fast()),
            rows: 0,
        })
    }

    pub fn write(&mut self, row: &Value) -> anyhow::Result<()> {
        serde_json::to_writer(&mut self.file, row)?;
        self.file.write_all(b"\n")?;
        self.rows += 1;
        if self.rows.is_multiple_of(ROWS_PER_FLUSH) {
            self.file.flush()?;
        }
        Ok(())
    }

    pub fn finish(self) -> anyhow::Result<()> {
        self.file.finish()?.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::io::Read as _;

    #[test]
    fn rows_come_back_in_order() {
        let path = std::env::temp_dir().join(format!("seat4-journal-{}", uuid::Uuid::now_v7()));
        let mut journal = Journal::create(&path).unwrap();
        journal.write(&serde_json::json!({"decision": 1})).unwrap();
        journal.write(&serde_json::json!({"decision": 2})).unwrap();
        journal.finish().unwrap();
        let mut text = String::new();
        GzDecoder::new(File::open(&path).unwrap())
            .read_to_string(&mut text)
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(text, "{\"decision\":1}\n{\"decision\":2}\n");
    }
}
