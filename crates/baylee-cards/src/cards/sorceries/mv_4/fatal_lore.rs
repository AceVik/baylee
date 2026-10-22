//! Fatal Lore — {2}{B}{B} — Sorcery
//! Oracle: An opponent chooses one —
//! Oracle: • You draw three cards.
//! Oracle: • You destroy up to two target creatures that player controls. They can't be regenerated. That player draws up to three cards.
//! Set: ALL #48 — Alliances | Scryfall ID: 24ba0b83-9671-4ee7-996d-57a3616b9c66 | Oracle ID: aeb85786-16fc-45d8-9383-60054a878c40
// NOT SUPPORTED: having an opponent choose the mode (CR 601.2b) and choosing "up to three cards" for the opponent's draw in mode 2 are not supported; the caster chooses the mode, and mode 2 destroys up to two target opponent creatures without regeneration and has each opponent draw three cards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FATAL_LORE,
    oracle_id = "aeb85786-16fc-45d8-9383-60054a878c40",
    scryfall_id = "24ba0b83-9671-4ee7-996d-57a3616b9c66",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "having an opponent choose the mode (CR 601.2b) and choosing 'up to three cards' for the opponent's draw in mode 2 are not supported; the caster chooses the mode, and mode 2 destroys up to two target opponent creatures without regeneration and has each opponent draw three cards",
    ),
    faces = &[face!(
        name = "Fatal Lore",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(&[Effect::draw(3)]),
            mode!(
                &[
                    Effect::Destroy {
                        target: TargetSpec::Object(&Filter::OPPONENT_CREATURE),
                        no_regen: true,
                    },
                    Effect::DrawCardsFor {
                        amount: Amount::Fixed(3),
                        who: PlayerRel::Opponent,
                    },
                ],
                targets = Some(TargetReq::up_to(
                    TargetSpec::Object(&Filter::OPPONENT_CREATURE),
                    2,
                )),
            ),
        ],
    }],
);
