//! Spell, draw, death, defense and run-income families.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Common drains one life on each noncreature spell you cast.
pub static TOWER_RUN_LANTERN_COMMON: TokenDef = TokenDef {
    name: "Ember Lantern — Common",
    abilities: &[triggered!(
        Trigger::SpellCast(&NONCREATURE_SPELL),
        &[Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: PlayerRel::EachOpponent
        }]
    )],
    ..TokenDef::DEFAULT
};
/// Uncommon drains two and adds upkeep selection.
pub static TOWER_RUN_LANTERN_UNCOMMON: TokenDef = TokenDef {
    name: "Ember Lantern — Uncommon",
    abilities: &[
        triggered!(
            Trigger::SpellCast(&NONCREATURE_SPELL),
            &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::EachOpponent
            }]
        ),
        UPKEEP_SCRY,
    ],
    ..TokenDef::DEFAULT
};
/// Rare drains three and heals with noncreature spells.
pub static TOWER_RUN_LANTERN_RARE: TokenDef = TokenDef {
    name: "Ember Lantern — Rare",
    abilities: &[
        triggered!(
            Trigger::SpellCast(&NONCREATURE_SPELL),
            &[Effect::LoseLife {
                amount: Amount::Fixed(3),
                target: PlayerRel::EachOpponent
            }]
        ),
        UPKEEP_SCRY,
        SPELL_LIFE,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic drains four and grows Plants from noncreature spells.
pub static TOWER_RUN_LANTERN_MYTHIC: TokenDef = TokenDef {
    name: "Ember Lantern — Mythic Rare",
    abilities: &[
        triggered!(
            Trigger::SpellCast(&NONCREATURE_SPELL),
            &[Effect::LoseLife {
                amount: Amount::Fixed(4),
                target: PlayerRel::EachOpponent
            }]
        ),
        UPKEEP_SCRY,
        SPELL_LIFE,
        SPELL_PLANT,
    ],
    ..TokenDef::DEFAULT
};

const DRAW_CHARGE: AbilityDef = triggered!(Trigger::Draws(PlayerRel::You), &[CHARGE_ONE]);
const THREE: Cost = cost!(RemoveCounterSelf {
    kind: CounterKind::Charge,
    n: 3
});
/// Common stores draws and spends three charges to scry one.
pub static TOWER_RUN_LENS_COMMON: TokenDef = TokenDef {
    name: "Tideglass Lens — Common",
    abilities: &[DRAW_CHARGE, paid(THREE, &[Effect::scry(1)], UNLIMITED)],
    ..TokenDef::DEFAULT
};
/// Uncommon scries two per payment and one each upkeep.
pub static TOWER_RUN_LENS_UNCOMMON: TokenDef = TokenDef {
    name: "Tideglass Lens — Uncommon",
    abilities: &[
        DRAW_CHARGE,
        paid(THREE, &[Effect::scry(2)], UNLIMITED),
        UPKEEP_SCRY,
    ],
    ..TokenDef::DEFAULT
};
/// Rare scries three per payment and offers a paid loot.
pub static TOWER_RUN_LENS_RARE: TokenDef = TokenDef {
    name: "Tideglass Lens — Rare",
    abilities: &[
        DRAW_CHARGE,
        paid(THREE, &[Effect::scry(3)], UNLIMITED),
        UPKEEP_SCRY,
        LOOT,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic scries four per payment and heals with each draw.
pub static TOWER_RUN_LENS_MYTHIC: TokenDef = TokenDef {
    name: "Tideglass Lens — Mythic Rare",
    abilities: &[
        DRAW_CHARGE,
        paid(THREE, &[Effect::scry(4)], UNLIMITED),
        UPKEEP_SCRY,
        LOOT,
        DRAW_LIFE,
    ],
    ..TokenDef::DEFAULT
};

const DEATH: Trigger = Trigger::Dies(&f!(your nontoken CREATURE));
/// Common drains one when your nontoken creatures die.
pub static TOWER_RUN_BELL_COMMON: TokenDef = TokenDef {
    name: "Grave Bell — Common",
    abilities: &[triggered!(
        DEATH,
        &[Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: PlayerRel::EachOpponent
        }]
    )],
    ..TokenDef::DEFAULT
};
/// Uncommon drains two and heals when your creatures die.
pub static TOWER_RUN_BELL_UNCOMMON: TokenDef = TokenDef {
    name: "Grave Bell — Uncommon",
    abilities: &[
        triggered!(
            DEATH,
            &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::EachOpponent
            }]
        ),
        DEATH_LIFE,
    ],
    ..TokenDef::DEFAULT
};
/// Rare drains three and adds a Plant each upkeep.
pub static TOWER_RUN_BELL_RARE: TokenDef = TokenDef {
    name: "Grave Bell — Rare",
    abilities: &[
        triggered!(
            DEATH,
            &[Effect::LoseLife {
                amount: Amount::Fixed(3),
                target: PlayerRel::EachOpponent
            }]
        ),
        DEATH_LIFE,
        UPKEEP_PLANT,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic drains four and draws on each of your nontoken creature deaths.
pub static TOWER_RUN_BELL_MYTHIC: TokenDef = TokenDef {
    name: "Grave Bell — Mythic Rare",
    abilities: &[
        triggered!(
            DEATH,
            &[Effect::LoseLife {
                amount: Amount::Fixed(4),
                target: PlayerRel::EachOpponent
            }]
        ),
        DEATH_LIFE,
        UPKEEP_PLANT,
        triggered!(DEATH, &[Effect::draw(1)]),
    ],
    ..TokenDef::DEFAULT
};

const SPELL_CHARGE: AbilityDef = triggered!(Trigger::SpellCast(&NONCREATURE_SPELL), &[CHARGE_ONE]);
const TWO: Cost = cost!(RemoveCounterSelf {
    kind: CounterKind::Charge,
    n: 2
});
const ONE: Cost = cost!(RemoveCounterSelf {
    kind: CounterKind::Charge,
    n: 1
});
/// Common stores noncreature spells; three charges buy mana of any color.
pub static TOWER_RUN_SPINDLE_COMMON: TokenDef = TokenDef {
    name: "Storm Spindle — Common",
    abilities: &[
        SPELL_CHARGE,
        mana(THREE, &[Effect::mana_of_any_color()], ONCE),
    ],
    ..TokenDef::DEFAULT
};
/// Uncommon needs two charges and charges each upkeep.
pub static TOWER_RUN_SPINDLE_UNCOMMON: TokenDef = TokenDef {
    name: "Storm Spindle — Uncommon",
    abilities: &[
        SPELL_CHARGE,
        mana(TWO, &[Effect::mana_of_any_color()], ONCE),
        UPKEEP_CHARGE,
    ],
    ..TokenDef::DEFAULT
};
/// Rare needs one charge and heals on noncreature spells.
pub static TOWER_RUN_SPINDLE_RARE: TokenDef = TokenDef {
    name: "Storm Spindle — Rare",
    abilities: &[
        SPELL_CHARGE,
        mana(ONE, &[Effect::mana_of_any_color()], ONCE),
        UPKEEP_CHARGE,
        SPELL_LIFE,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic removes the turn cap and grows Plants from noncreature spells.
pub static TOWER_RUN_SPINDLE_MYTHIC: TokenDef = TokenDef {
    name: "Storm Spindle — Mythic Rare",
    abilities: &[
        SPELL_CHARGE,
        mana(ONE, &[Effect::mana_of_any_color()], UNLIMITED),
        UPKEEP_CHARGE,
        SPELL_LIFE,
        SPELL_PLANT,
    ],
    ..TokenDef::DEFAULT
};

const VIGILANCE: AbilityDef = static_ability!(
    Filter::YOUR_CREATURE,
    Modifier::AddKeyword(KeywordSet::VIGILANCE)
);
const LIFELINK: AbilityDef = static_ability!(
    Filter::YOUR_CREATURE,
    Modifier::AddKeyword(KeywordSet::LIFELINK)
);
const HEXPROOF: AbilityDef = static_ability!(
    Filter::YOUR_CREATURE,
    Modifier::AddKeyword(KeywordSet::HEXPROOF)
);
/// Common gives your creatures +0/+1.
pub static TOWER_RUN_AEGIS_COMMON: TokenDef = TokenDef {
    name: "Dawn Aegis — Common",
    abilities: &[static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::ModifyPT(0, 1)
    )],
    ..TokenDef::DEFAULT
};
/// Uncommon gives +0/+2 and vigilance.
pub static TOWER_RUN_AEGIS_UNCOMMON: TokenDef = TokenDef {
    name: "Dawn Aegis — Uncommon",
    abilities: &[
        static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(0, 2)),
        VIGILANCE,
    ],
    ..TokenDef::DEFAULT
};
/// Rare gives +0/+3 and adds lifelink.
pub static TOWER_RUN_AEGIS_RARE: TokenDef = TokenDef {
    name: "Dawn Aegis — Rare",
    abilities: &[
        static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(0, 3)),
        VIGILANCE,
        LIFELINK,
    ],
    ..TokenDef::DEFAULT
};
/// Mythic gives +0/+4 and adds hexproof.
pub static TOWER_RUN_AEGIS_MYTHIC: TokenDef = TokenDef {
    name: "Dawn Aegis — Mythic Rare",
    abilities: &[
        static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(0, 4)),
        VIGILANCE,
        LIFELINK,
        HEXPROOF,
    ],
    ..TokenDef::DEFAULT
};

/// Common Acorn's run-income bonus is settled by the verified authority.
pub static TOWER_RUN_ACORN_COMMON: TokenDef = TokenDef {
    name: "Gilded Acorn — Common",
    ..TokenDef::DEFAULT
};
/// Uncommon increases income and adds upkeep healing.
pub static TOWER_RUN_ACORN_UNCOMMON: TokenDef = TokenDef {
    name: "Gilded Acorn — Uncommon",
    abilities: &[UPKEEP_LIFE],
    ..TokenDef::DEFAULT
};
/// Rare increases income and adds a Plant each upkeep.
pub static TOWER_RUN_ACORN_RARE: TokenDef = TokenDef {
    name: "Gilded Acorn — Rare",
    abilities: &[UPKEEP_LIFE, UPKEEP_PLANT],
    ..TokenDef::DEFAULT
};
/// Mythic adds victory income and strengthens your creature tokens.
pub static TOWER_RUN_ACORN_MYTHIC: TokenDef = TokenDef {
    name: "Gilded Acorn — Mythic Rare",
    abilities: &[UPKEEP_LIFE, UPKEEP_PLANT, TOKEN_GROWTH],
    ..TokenDef::DEFAULT
};
