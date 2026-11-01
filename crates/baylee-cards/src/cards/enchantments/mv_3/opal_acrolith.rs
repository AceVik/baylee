//! Opal Acrolith — {2}{W} — Enchantment
//! Oracle: Whenever an opponent casts a creature spell, if this permanent is an enchantment, it becomes a 2/4 Soldier creature.
//! Oracle: {0}: This permanent becomes an enchantment.
//! Set: USG #22 — Urza's Saga | Scryfall ID: 839b4c10-f68f-4321-82ee-5ec257f63866 | Oracle ID: d08b8dd6-3a44-4e86-8ce8-2d0a014a9aa8
// PARTIAL — becoming a plain enchantment with no retained card types or creature subtypes is not supported; retains only the opponent-creature-spell animation

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);

card!(
    index = index::OPAL_ACROLITH,
    oracle_id = "d08b8dd6-3a44-4e86-8ce8-2d0a014a9aa8",
    scryfall_id = "839b4c10-f68f-4321-82ee-5ec257f63866",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "becoming a plain enchantment with no retained card types or creature subtypes is not supported; retains only the opponent-creature-spell animation",
    ),
    faces = &[face!(
        name = "Opal Acrolith",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::SpellCast(&OPPONENT_CREATURE_SPELL),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::BecomeType {
                        types: TypeSet::CREATURE,
                        subtype: subtypes::creature::SOLDIER,
                    },
                    Duration::Indefinitely,
                ),
                Effect::continuous(&Filter::This, Modifier::SetPT(2, 4), Duration::Indefinitely),
            ],
            condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
        ),
        // NOT SUPPORTED: {0}: This permanent becomes an enchantment;
        // AddType/RemoveType would retain unrelated types and its Soldier subtype.
    ],
);
