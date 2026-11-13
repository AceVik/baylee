//! Balduvian Dead — {3}{B} — Creature — Zombie
//! Oracle: {2}{R}, Exile a creature card from your graveyard: Create a 3/1 black and red Graveborn creature token with haste. Sacrifice it at the beginning of the next end step.
//! Set: ME2 #79 — Masters Edition II | Scryfall ID: c9aee135-65f6-4abb-b0ca-ee5ceb958a24 | Oracle ID: 18d6b89a-3588-43d9-a247-4747a7d784fc
// NOT SUPPORTED: sacrificing the Graveborn token at the beginning of the next end step is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BALDUVIAN_DEAD,
    oracle_id = "18d6b89a-3588-43d9-a247-4747a7d784fc",
    scryfall_id = "c9aee135-65f6-4abb-b0ca-ee5ceb958a24",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "'{2}{R}, Exile a creature card from your graveyard: create a 3/1 Graveborn token with haste. Sacrifice it at the beginning of the next end step' is not supported: no effect names the token it just created for a delayed end-step sacrifice (the token would stay for good, which is more than printed), so the ability is left off and the card is a 2/3 creature",
    ),
    faces = &[face!(
        name = "Balduvian Dead",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(3),
    ),],
    // NOT SUPPORTED: "{2}{R}, Exile a creature card from your graveyard: Create a 3/1 black and red Graveborn creature token with haste. Sacrifice it at the beginning of the next end step." `AtNextEndStep` names the source or the first target, not a token the ability created, and sacrificing every Graveborn token at the end step would also take the permanent ones Sek'Kuar, Deathkeeper makes. A token that stayed would be more than printed.
);
