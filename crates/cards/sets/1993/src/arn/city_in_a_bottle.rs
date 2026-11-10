//! City in a Bottle — {2} — Artifact
//! Oracle: Whenever one or more other nontoken permanents with a name originally printed in the Arabian Nights expansion are on the battlefield, their controllers sacrifice them.
//! Oracle: Players can't cast spells or play lands with a name originally printed in the Arabian Nights expansion.
//! Set: VMA #265 — Vintage Masters | Scryfall ID: cfd5c243-2a40-4ba8-ad00-715f52eeda62 | Oracle ID: a83f25e3-4d84-4c9b-ab12-19b8d326e459

use baylee_cards_dsl::prelude::*;

static ARABIAN_NIGHTS_CARD: Filter =
    Filter::NameOriginallyPrintedIn(OriginalExpansion::ArabianNights);

static NONTOKEN_ARABIAN_NIGHTS_EXCEPT_SELF: Filter = Filter::And(&[
    Filter::Another,
    Filter::Not(&Filter::IsToken),
    Filter::NameOriginallyPrintedIn(OriginalExpansion::ArabianNights),
]);

static OTHER_ARABIAN_NIGHTS_PRESENT: Condition =
    Condition::BattlefieldCount(&NONTOKEN_ARABIAN_NIGHTS_EXCEPT_SELF, 1);

card!(
    index = index::CITY_IN_A_BOTTLE,
    oracle_id = "a83f25e3-4d84-4c9b-ab12-19b8d326e459",
    scryfall_id = "cfd5c243-2a40-4ba8-ad00-715f52eeda62",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "City in a Bottle",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::State(&OTHER_ARABIAN_NIGHTS_PRESENT),
            &[Effect::SacrificeAll {
                filter: &NONTOKEN_ARABIAN_NIGHTS_EXCEPT_SELF,
            }],
        ),
        static_ability!(
            Filter::This,
            Modifier::PlayersCantPlay(&ARABIAN_NIGHTS_CARD)
        ),
    ],
);
