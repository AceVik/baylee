//! City in a Bottle — {2} — Artifact
//! Oracle: Whenever one or more other nontoken permanents with a name originally printed in the Arabian Nights expansion are on the battlefield, their controllers sacrifice them.
//! Oracle: Players can't cast spells or play lands with a name originally printed in the Arabian Nights expansion.
//! Set: VMA #265 — Vintage Masters | Scryfall ID: cfd5c243-2a40-4ba8-ad00-715f52eeda62 | Oracle ID: a83f25e3-4d84-4c9b-ab12-19b8d326e459
// PARTIAL — both sentences are off the card: nothing names a set, no effect
// sacrifices every matching permanent, and no static bans every player's
// casts or any land play.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITY_IN_A_BOTTLE,
    oracle_id = "a83f25e3-4d84-4c9b-ab12-19b8d326e459",
    scryfall_id = "cfd5c243-2a40-4ba8-ad00-715f52eeda62",
    faces = &[face!(
        name = "City in a Bottle",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "both sentences need a filter for a name originally printed in the \
         Arabian Nights expansion, and no Filter or Condition names a set of \
         printings; the first also needs an effect that sacrifices every \
         permanent a filter matches, and the second a cast prohibition over \
         every player and a land-play prohibition, neither of which exists"
    ),
    // NOT SUPPORTED: "Whenever one or more other nontoken permanents with a
    // name originally printed in the Arabian Nights expansion are on the
    // battlefield, their controllers sacrifice them." — `Trigger::State`
    // with `Condition::BattlefieldCount` can ask whether permanents are on
    // the battlefield, but no `Filter` names the set: `Filter::Named` matches
    // one name and nothing expands a set of printings. There is also no
    // "each player sacrifices every matching permanent" effect —
    // `Effect::SacrificeFilter` buys one per player, their choice.
    // NOT SUPPORTED: "Players can't cast spells or play lands with a name
    // originally printed in the Arabian Nights expansion." — the set filter
    // is missing as above; `Modifier::OpponentsCantCast` reaches only the
    // effect's opponents where this sentence binds the artifact's controller
    // too, and no `Modifier` forbids playing a land at all.
    abilities = &[],
);
