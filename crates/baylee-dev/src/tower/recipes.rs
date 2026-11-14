//! Small independent recipe oracle for conformance fixtures, not an inventory store.
//! Ownership, transactions, equip limits and payment must be tested by the future adapter.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Mythic,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Piece {
    pub family: String,
    pub definition: u32,
    pub rarity: Rarity,
    pub gold_basis: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    /// Selected ingredients, not a claim that the caller owns them.
    pub pieces: Vec<Piece>,
    /// Gold bases of selected Mythic Potions, not ordinary duel potions.
    pub potion_bases: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Output {
    pub rarity: Rarity,
    pub gold_basis: u32,
}

pub(super) fn quote(input: &Input) -> Option<Output> {
    let first = input.pieces.first()?;
    if !super::bounded_label(&first.family) || first.definition == 0 {
        return None;
    }
    let rarity = match (first.rarity, input.pieces.len(), input.potion_bases.len()) {
        (Rarity::Common, 3, 0) => Rarity::Uncommon,
        (Rarity::Uncommon, 3, 0) => Rarity::Rare,
        (Rarity::Rare, 1, 1) => Rarity::Mythic,
        _ => return None,
    };
    if input.pieces.iter().any(|piece| {
        piece.family != first.family
            || piece.definition != first.definition
            || piece.rarity != first.rarity
    }) {
        return None;
    }
    let gold_basis = input
        .pieces
        .iter()
        .map(|p| p.gold_basis)
        .chain(input.potion_bases.iter().copied())
        .try_fold(0_u32, u32::checked_add)?;
    Some(Output { rarity, gold_basis })
}

pub(super) fn resale(gold_basis: u32, catalog_ceiling: u32) -> u32 {
    (gold_basis / 2).min(catalog_ceiling)
}
