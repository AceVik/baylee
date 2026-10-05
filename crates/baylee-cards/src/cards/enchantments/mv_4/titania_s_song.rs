//! Titania's Song — {3}{G} — Enchantment
//! Oracle: Each noncreature artifact loses all abilities and becomes an artifact creature with power and toughness each equal to its mana value. If this enchantment leaves the battlefield, this effect continues until end of turn.
//! Set: ME4 #170 — Masters Edition IV | Scryfall ID: 4bb135e7-f9e3-4abc-8374-180316498fc3 | Oracle ID: 30ac0f06-3dd3-4827-8136-4cf8adbf9b12
// PARTIAL — the animation (artifact creature, power and toughness each equal
// to mana value) and its continuation until end of turn are written; "loses
// all abilities" is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TITANIA_S_SONG,
    oracle_id = "30ac0f06-3dd3-4827-8136-4cf8adbf9b12",
    scryfall_id = "4bb135e7-f9e3-4abc-8374-180316498fc3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no modifier both animates a noncreature artifact and strips its \
         abilities: `LoseAllAbilities` reads its filter at layer 6, after \
         `AnimateNoncreatureArtifact` has added CREATURE at layer 4, so a \
         not-a-creature filter can no longer match, so the ability loss is off"
    ),
    faces = &[face!(
        name = "Titania's Song",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: "loses all abilities" — `Modifier::LoseAllAbilities` on
    // `Filter::ARTIFACT` would strip artifact creatures too, and on
    // `Filter::And(&[Filter::ARTIFACT, Filter::Not(&Filter::CREATURE)])` it
    // matches nothing by layer 6, because the animation has already added
    // CREATURE. No combined animate-and-strip variant exists, so only the
    // animation and its continuation are written.
    abilities = &[
        static_ability!(Filter::ARTIFACT, Modifier::AnimateNoncreatureArtifact),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::continuous(
                &Filter::ARTIFACT,
                Modifier::AnimateNoncreatureArtifact,
                Duration::UntilEndOfTurn
            )]
        ),
    ],
);
