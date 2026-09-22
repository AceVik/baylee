//! Twists and Turns // Mycoid Maze — {G} — Enchantment // Land — Cave
//! Oracle: If a creature you control would explore, instead you scry 1, then that creature explores.
//! Oracle: When this enchantment enters, target creature you control explores.
//! Oracle: When a land you control enters, if you control seven or more lands, transform this enchantment.
//! Oracle: (Transforms from Twists and Turns.)
//! Oracle: {T}: Add {G}.
//! Oracle: {3}{G}, {T}: Look at the top four cards of your library. You may reveal a creature card from among them and put that card into your hand. Put the rest on the bottom of your library in a random order.
//! Set: LCI #217 — The Lost Caverns of Ixalan | Scryfall ID: 3cdf691e-96a5-45c7-9b94-6f04af81c8e4 | Oracle ID: 740aa9d9-91a9-431e-8bf9-1344e5273e27
//! Face: Twists and Turns — {G} — Enchantment
//! Face: Mycoid Maze —  — Land — Cave
// PARTIAL — the landfall transform trigger and Mycoid Maze's {T}: Add {G} are
// built; explore and the back face's filtered look-at-top are not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Mycoid Maze's own abilities. A face states its own, the way it states its
/// own keywords: the front face's trigger is on the card, the back face's
/// mana ability is here.
static MYCOID_MAZE_ABILITIES: &[AbilityDef] =
    &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card!(
    index = index::TWISTS_AND_TURNS,
    oracle_id = "740aa9d9-91a9-431e-8bf9-1344e5273e27",
    scryfall_id = "3cdf691e-96a5-45c7-9b94-6f04af81c8e4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "explore is not expressible (no Effect explores, and ReplacementRule \
         has no explore variant), and the back face's look-at-top takes no \
         filter and is not optional"
    ),
    faces = &[
        face!(
            name = "Twists and Turns",
            mana_cost = mana!("{G}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Mycoid Maze",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::CAVE],
            // A transforming back is only ever reached by turning the card
            // over (CR 712.2), so it is not a face anybody casts.
            castable_from_hand = false,
            // NOT SUPPORTED: "{3}{G}, {T}: Look at the top four cards of your
            // library. You may reveal a creature card from among them and put
            // that card into your hand. Put the rest on the bottom of your
            // library in a random order." — `Effect::LookAtTopPick` takes no
            // filter (it keeps any card, not a creature card), is not
            // optional, and bottoms the rest in any order rather than at
            // random.
            abilities = MYCOID_MAZE_ABILITIES,
        ),
    ],
    abilities = &[
        // NOT SUPPORTED: "If a creature you control would explore, instead you
        // scry 1, then that creature explores." — `ReplacementRule` has no
        // explore variant, and there is no explore effect for it to precede.
        // NOT SUPPORTED: "When this enchantment enters, target creature you
        // control explores." — no `Effect` explores.
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::ExileSelfReturnAsFace { face: 1 }],
            condition = Some(Condition::ControlCount(&Filter::LAND, 7)),
        ),
    ],
);
