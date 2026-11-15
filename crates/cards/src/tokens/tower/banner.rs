//! Original Swarm Banner: only creature tokens you control receive its support.
use baylee_cards_dsl::prelude::*;

/// Common: your creature tokens get +1/+0 while this artifact remains.
pub static TOWER_BANNER_COMMON: TokenDef = TokenDef {
    name: "Swarm Banner — Common",
    types: TypeSet::ARTIFACT,
    abilities: &[static_ability!(
        f!(your token CREATURE),
        Modifier::ModifyPT(1, 0)
    )],
    ..TokenDef::DEFAULT
};

/// Uncommon: your creature tokens get +1/+1.
pub static TOWER_BANNER_UNCOMMON: TokenDef = TokenDef {
    name: "Swarm Banner — Uncommon",
    types: TypeSet::ARTIFACT,
    abilities: &[static_ability!(
        f!(your token CREATURE),
        Modifier::ModifyPT(1, 1)
    )],
    ..TokenDef::DEFAULT
};

/// Rare: your creature tokens get +2/+1.
pub static TOWER_BANNER_RARE: TokenDef = TokenDef {
    name: "Swarm Banner — Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[static_ability!(
        f!(your token CREATURE),
        Modifier::ModifyPT(2, 1)
    )],
    ..TokenDef::DEFAULT
};

/// Mythic Rare: your creature tokens get +2/+2 and trample.
pub static TOWER_BANNER_MYTHIC: TokenDef = TokenDef {
    name: "Swarm Banner — Mythic Rare",
    types: TypeSet::ARTIFACT,
    abilities: &[
        static_ability!(f!(your token CREATURE), Modifier::ModifyPT(2, 2)),
        static_ability!(
            f!(your token CREATURE),
            Modifier::AddKeyword(KeywordSet::TRAMPLE)
        ),
    ],
    ..TokenDef::DEFAULT
};
