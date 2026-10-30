use super::*;
use std::{collections::BTreeMap, fs};

pub(super) struct Catalog {
    cards: BTreeMap<u64, usize>,
    front: Vec<f32>,
    back: Vec<f32>,
    tokens: Vec<f32>,
    pub roots: BTreeMap<String, u32>,
    pub structure: Value,
}
fn table(path: &Path, rows: usize) -> Result<Vec<f32>> {
    ensure!(
        rows <= MAX_BYTES / 640,
        "research DSL catalog exceeds memory bound"
    );
    ensure!(
        path.metadata()?.len()
            == u64::try_from(rows.checked_mul(640).context("catalog size overflow")?)?,
        "catalog table dimensions differ"
    );
    let bytes = fs::read(path)?;
    let values = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect::<Vec<_>>();
    ensure!(
        values.iter().all(|f| f.is_finite()),
        "nonfinite catalog table"
    );
    Ok(values)
}
impl Catalog {
    pub fn load(root: &Path, metadata: &Value) -> Result<Self> {
        let cat = &metadata["source_conversion_manifest"]["catalog"];
        ensure!(cat["dsl_version"] == 2, "unsupported research DSL version");
        let list = array(cat, "cards")?;
        let token_rows = uint(&cat["tokens"])?
            .checked_add(1)
            .context("token count overflow")?;
        ensure!(
            list.len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(token_rows))
                .is_some_and(|rows| rows <= MAX_BYTES / 640),
            "combined research DSL catalog exceeds memory bound"
        );
        let mut cards = BTreeMap::new();
        for (row, card) in list.iter().enumerate() {
            let card = card.as_u64().context("catalog card ID")?;
            ensure!(
                card < u64::from(u32::MAX) && cards.insert(card, row).is_none(),
                "duplicate/invalid catalog card"
            );
        }
        let front = table(&root.join("card_dsl.bin"), cards.len())?;
        let back = table(&root.join("card_dsl_back.bin"), cards.len())?;
        let tokens = table(&root.join("token_dsl.bin"), token_rows)?;
        let structure = crate::files::read_json(&root.join("card_structure.json"))?;
        ensure!(
            structure["version"] == 1,
            "unsupported research structure schema"
        );
        ensure!(
            sha(&serde_json::to_vec(&structure)?) == cat["structure_sha256"],
            "research structure semantic digest differs"
        );
        let roots = serde_json::from_value(structure["roots"].clone())?;
        Ok(Self {
            cards,
            front,
            back,
            tokens,
            roots,
            structure,
        })
    }
    pub fn entity(&self, card: u64, face: u64, token: u64) -> Result<Vec<f32>> {
        ensure!(
            u16::try_from(token).is_ok(),
            "entity token slot exceeds native representation"
        );
        ensure!(
            matches!(face, 0 | 1 | 255),
            "research catalog represents only front/back faces"
        );
        if card == u64::from(u32::MAX) {
            if token == 0 {
                return Ok(vec![0.; 160]);
            }
            let row = usize::try_from(token)?;
            ensure!(row < self.tokens.len() / 160, "unrepresented token DSL");
            return Ok(self.tokens[row * 160..(row + 1) * 160].to_vec());
        }
        // Copied token objects can retain their token slot while using active card rules.
        // Native inference prioritizes the known active card over token fallback.
        self.card(card, face)
    }
    pub fn card(&self, card: u64, face: u64) -> Result<Vec<f32>> {
        ensure!(
            matches!(face, 0 | 1 | 255),
            "research catalog represents only front/back faces"
        );
        let row = *self
            .cards
            .get(&card)
            .context("unrepresented research catalog card")?;
        let table = if face == 1 { &self.back } else { &self.front };
        Ok(table[row * 160..(row + 1) * 160].to_vec())
    }
}
