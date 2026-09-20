//! Opal Palace — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color in your commander's color identity. If you spend this mana to cast your commander, it enters with a number of additional +1/+1 counters on it equal to the number of times it's been cast from the command zone this game.
//! Set: SOC #390 — Secrets of Strixhaven Commander | Scryfall ID: 912553e7-1e67-4045-84fd-0a791754cf6c | Oracle ID: aa6723a2-75da-49f5-a1ba-cbfa82c55301
// PARTIAL — both mana abilities are built; the second one's spend rider is
// not expressible, so the card cannot claim Implemented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OPAL_PALACE,
    oracle_id = "aa6723a2-75da-49f5-a1ba-cbfa82c55301",
    scryfall_id = "912553e7-1e67-4045-84fd-0a791754cf6c",
    faces = &[face!(name = "Opal Palace", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "spend rider: a commander cast with this mana enters with extra +1/+1 \
         counters — no SpendRider variant carries it, and no filter names your commander"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: If you spend this mana to cast your commander, it
        // enters with a number of additional +1/+1 counters on it equal to
        // the number of times it's been cast from the command zone this game.
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_commander_identity()]),
    ],
);
