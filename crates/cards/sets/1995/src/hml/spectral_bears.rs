//! Spectral Bears — {1}{G} — Creature — Bear Spirit
//! Oracle: Whenever this creature attacks, if defending player controls no black nontoken permanents, it doesn't untap during your next untap step.
//! Set: ME1 #131 — Masters Edition | Scryfall ID: e56143db-0274-4878-9d54-619e7f037a33 | Oracle ID: 539899f3-f3f2-4f7a-a6f6-e80d4a39f753
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const DEFENDERS_BLACK_NONTOKEN_PERMANENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
    Filter::Not(&Filter::IsToken),
    Filter::ControlledByDefendingPlayer,
]);

card!(
    index = index::SPECTRAL_BEARS,
    oracle_id = "539899f3-f3f2-4f7a-a6f6-e80d4a39f753",
    scryfall_id = "e56143db-0274-4878-9d54-619e7f037a33",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spectral Bears",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAR, subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::ModifySource {
            modifier: Modifier::DoesNotUntap,
            duration: Duration::UntilYourNextUntapStep,
        }],
        condition = Some(Condition::BattlefieldCountAtMost(
            &DEFENDERS_BLACK_NONTOKEN_PERMANENT,
            0
        )),
    )],
);
