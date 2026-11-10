//! Blinding Beam — {2}{W} — Instant
//! Oracle: Choose one —
//! Oracle: • Tap two target creatures.
//! Oracle: • Creatures don't untap during target player's next untap step.
//! Oracle: Entwine {1} (Choose both if you pay the entwine cost.)
//! Set: MMA #7 — Modern Masters | Scryfall ID: d0b15737-1026-407f-b2a8-4f7932b2bb18 | Oracle ID: 0766cf02-ea55-4271-8c73-213c93b0baac
use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLINDING_BEAM,
    oracle_id = "0766cf02-ea55-4271-8c73-213c93b0baac",
    scryfall_id = "d0b15737-1026-407f-b2a8-4f7932b2bb18",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Blinding Beam",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
        entwine = Some(cost!("{1}")),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::TapTarget],
                targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2)),
            ),
            mode!(
                &[Effect::TargetPlayersCreaturesDontUntapNextUntap],
                targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
            ),
        ],
    }],
);
