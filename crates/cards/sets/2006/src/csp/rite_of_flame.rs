//! Rite of Flame — {R} — Sorcery
//! Oracle: Add {R}{R}, then add {R} for each card named Rite of Flame in each graveyard.
//! Set: CSP #96 — Coldsnap | Scryfall ID: c062caf7-f0eb-44db-9f74-e6711a13fada | Oracle ID: 8a2e53f9-8100-488f-8504-b59e9bd1cc29

use baylee_cards_dsl::prelude::*;

/// {R}{R}, then one more for each card named Rite of Flame in every
/// graveyard. The spell is on the stack while it resolves (CR 608.2n puts
/// it in the graveyard only as the last step), so it never counts itself.
static RED: Amount = Amount::Plus {
    base: &Amount::CountOf {
        filter: &Filter::Named("Rite of Flame"),
        zone: ZoneSel::GraveyardAll,
    },
    offset: 2,
};

card!(
    index = index::RITE_OF_FLAME,
    oracle_id = "8a2e53f9-8100-488f-8504-b59e9bd1cc29",
    scryfall_id = "c062caf7-f0eb-44db-9f74-e6711a13fada",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rite of Flame",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::mana_dynamic(ManaColor::Red, RED)])],
);
