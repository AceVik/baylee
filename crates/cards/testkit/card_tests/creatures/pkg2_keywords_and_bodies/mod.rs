//! Package 2 behavioral card tests: vanilla, French-vanilla, keyword,
//! combat-restriction, protection, and enter-modifier creatures (786 cards).
//!
//! Every test is derived from Scryfall Oracle text, mana cost, type line,
//! colors, and power/toughness, exercising regular gameplay actions.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;

mod cast_and_arrive_a;
mod cast_and_arrive_b;
mod cast_and_arrive_c;
mod evasion_and_combat;
mod keywords_and_protection;

#[allow(clippy::struct_excessive_bools)]
pub(super) struct ArrivalRow {
    pub card: CardIndex,
    pub cost: &'static [(ManaColor, u32)],
    pub alt_cost: Option<&'static [(ManaColor, u32)]>,
    pub nonzero_cost: bool,
    pub power: i16,
    pub toughness: i16,
    pub enters_tapped: bool,
    pub plus_counters: u16,
    pub is_legendary: bool,
    pub is_artifact: bool,
    pub is_enchantment: bool,
    pub colors: &'static [Color],
}

pub(super) fn colors_of(e: &Engine<RegistryLookup>, id: ObjectId) -> ColorSet {
    e.state().object(id).unwrap().characteristics().colors
}

pub(super) fn supertypes_of(e: &Engine<RegistryLookup>, id: ObjectId) -> SupertypeSet {
    e.state().object(id).unwrap().characteristics().supertypes
}

pub(super) fn types_of(e: &Engine<RegistryLookup>, id: ObjectId) -> TypeSet {
    e.state().object(id).unwrap().characteristics().types
}

pub(super) fn has_kw(e: &Engine<RegistryLookup>, id: ObjectId, k: KeywordSet) -> bool {
    keywords(e, id).contains(k)
}

#[track_caller]
#[allow(clippy::too_many_lines)]
pub(super) fn check_arrival_rows(rows: &[ArrivalRow], base_seed: u64) {
    for (n, row) in rows.iter().enumerate() {
        let mut e = game(base_seed + (n as u64) * 2, &[], &[], &[row.card], &[]);
        if row.nonzero_cost {
            assert!(
                !castable(&e, P0, row.card),
                "row {n}: should not be castable with an empty mana pool"
            );
        }
        float(&mut e, P0, row.cost);
        assert!(
            castable(&e, P0, row.card),
            "row {n}: should be castable when printed cost is in pool"
        );
        cast_with_floating(&mut e, P0, row.card);
        settle(&mut e);
        assert_eq!(
            pool_total(&e, P0),
            0,
            "row {n}: entire printed cost was spent"
        );

        // Zero-toughness creatures (Force of Savagery, 8/0) enter and die to SBA
        // unless an anthem boosts their toughness (CR 704.5f).
        if row.toughness <= 0 {
            assert!(
                in_graveyard(&e, P0, row.card).is_some(),
                "row {n}: 0-toughness creature must die to SBA on an empty board"
            );
            let mut e_anthem = game(
                base_seed + (n as u64) * 2 + 1,
                &[ids::jacques_le_vert()],
                &[],
                &[row.card],
                &[],
            );
            float(&mut e_anthem, P0, row.cost);
            cast_with_floating(&mut e_anthem, P0, row.card);
            settle(&mut e_anthem);
            let id = on_battlefield(&e_anthem, P0, row.card)
                .unwrap_or_else(|| panic!("row {n}: creature with anthem never arrived"));
            assert_eq!(
                pt(&e_anthem, id),
                (row.power, row.toughness + 2),
                "row {n}: battlefield P/T under Jacques le Vert (+0/+2)"
            );
            assert_eq!(
                colors_of(&e_anthem, id),
                ColorSet::from_slice(row.colors),
                "row {n}: colors mismatch"
            );
            continue;
        }

        let id = on_battlefield(&e, P0, row.card)
            .unwrap_or_else(|| panic!("row {n}: creature never arrived on battlefield"));
        assert_eq!(
            pt(&e, id),
            (row.power, row.toughness),
            "row {n}: battlefield P/T mismatch"
        );
        assert_eq!(
            tapped(&e, id),
            row.enters_tapped,
            "row {n}: enters-tapped status mismatch"
        );
        assert_eq!(
            counters_on(&e, id, CounterKind::P1P1),
            row.plus_counters,
            "row {n}: +1/+1 counter count mismatch"
        );
        assert_eq!(
            supertypes_of(&e, id).contains(SupertypeSet::LEGENDARY),
            row.is_legendary,
            "row {n}: legendary supertype mismatch"
        );
        let t = types_of(&e, id);
        assert!(t.contains(TypeSet::CREATURE), "row {n}: must be a creature");
        assert_eq!(
            t.contains(TypeSet::ARTIFACT),
            row.is_artifact,
            "row {n}: artifact type mismatch"
        );
        assert_eq!(
            t.contains(TypeSet::ENCHANTMENT),
            row.is_enchantment,
            "row {n}: enchantment type mismatch"
        );
        assert_eq!(
            colors_of(&e, id),
            ColorSet::from_slice(row.colors),
            "row {n}: colors mismatch"
        );

        if let Some(alt) = row.alt_cost {
            let mut e_alt = game(base_seed + (n as u64) * 2 + 1, &[], &[], &[row.card], &[]);
            float(&mut e_alt, P0, alt);
            assert!(
                castable(&e_alt, P0, row.card),
                "row {n}: should be castable with alternative hybrid payment"
            );
            cast_with_floating(&mut e_alt, P0, row.card);
            settle(&mut e_alt);
            let id_alt = on_battlefield(&e_alt, P0, row.card)
                .unwrap_or_else(|| panic!("row {n}: creature never arrived off alt hybrid cost"));
            assert_eq!(pool_total(&e_alt, P0), 0, "row {n}: alt hybrid cost spent");
            assert_eq!(
                pt(&e_alt, id_alt),
                (row.power, row.toughness),
                "row {n}: alt P/T"
            );
        }
    }
}
