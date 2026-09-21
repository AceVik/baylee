//! Birgi, God of Storytelling // Harnfel, Horn of Bounty — {2}{R} — Legendary Creature — God // Legendary Artifact
//! Oracle: Whenever you cast a spell, add {R}. Until end of turn, you don't lose this mana as steps and phases end.
//! Oracle: Creatures you control can boast twice during each of your turns rather than once.
//! Oracle: Discard a card: Exile the top two cards of your library. You may play those cards this turn.
//! Set: KHM #123 — Kaldheim | Scryfall ID: 44657ab1-0a6a-4a5f-9688-86f239083821 | Oracle ID: fb81e4d3-1d8c-4779-be62-87cf49277e51
//! Face: Birgi, God of Storytelling — {2}{R} — Legendary Creature — God
//! Face: Harnfel, Horn of Bounty — {4}{R} — Legendary Artifact
// PARTIAL — Birgi's "whenever you cast a spell, add {R}" is built; the rest
// of the card has no spelling in the DSL yet.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BIRGI_GOD_OF_STORYTELLING,
    oracle_id = "fb81e4d3-1d8c-4779-be62-87cf49277e51",
    scryfall_id = "44657ab1-0a6a-4a5f-9688-86f239083821",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Birgi, God of Storytelling",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::GOD],
            power = Some(3),
            toughness = Some(3),
        ),
        face!(
            name = "Harnfel, Horn of Bounty",
            mana_cost = mana!("{4}{R}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
    ],
    coverage = Coverage::Partial(
        "the spell mana's mana-retention rider, the boast clause and Harnfel's \
         exile-the-top-two-and-play-them ability have no DSL variant"
    ),
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::ControlledByYou),
        &[Effect::mana(ManaColor::Red, 1)]
    )],
);

// NOT SUPPORTED: "Until end of turn, you don't lose this mana as steps and
// phases end." — no `Effect` or `Modifier` keeps mana in a pool past a step
// or phase, so the {R} this trigger adds empties with the step.
// NOT SUPPORTED: "Creatures you control can boast twice during each of your
// turns rather than once." — boast is not a keyword bit the engine reads, and
// no `Modifier` changes how often a printed ability may be activated.
// NOT SUPPORTED: "Discard a card: Exile the top two cards of your library.
// You may play those cards this turn." — no `Effect` exiles the top card of a
// library and no permission modifier grants playing cards from exile, so the
// ability is left off rather than built with half of its effect missing.
