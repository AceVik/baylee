//! Shared effect storage; 640 definitions borrow these forty immutable rewards.
use super::{Amount, Duration, Effect, Filter, KeywordSet, PlayerRel, Vessel};

const fn effect(vessel: Vessel, tier: usize) -> Effect {
    let n = match vessel.level(tier) {
        Some(level) => level.amount as u32,
        None => panic!("known tier"),
    };
    match vessel {
        Vessel::Chalice => Effect::gain_life(n),
        Vessel::Scepter => Effect::LoseLife {
            amount: Amount::Fixed(n),
            target: PlayerRel::EachOpponent,
        },
        Vessel::Prism => Effect::scry(n),
        Vessel::Seed => Effect::CreateTokensFor {
            player: PlayerRel::You,
            token: &super::super::super::TOWER_SEEDLING,
            amount: Amount::Fixed(n),
        },
        Vessel::Tome | Vessel::Lens => Effect::draw(1),
        Vessel::Horn => Effect::CreateTokensFor {
            player: PlayerRel::You,
            token: &crate::tokens::SOLDIER_1_1_WHITE,
            amount: Amount::Fixed(n),
        },
        Vessel::Flask => Effect::mana_of_any_color(),
        Vessel::Bulwark | Vessel::Banner => Effect::PumpFilter {
            filter: &Filter::YOUR_CREATURE,
            controlled_by: None,
            power: Amount::Fixed(if matches!(vessel, Vessel::Banner) {
                n
            } else {
                0
            }),
            toughness: Amount::Fixed(if matches!(vessel, Vessel::Bulwark) {
                n
            } else {
                0
            }),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        },
    }
}

static EFFECTS: [[Effect; 4]; 10] = {
    let mut effects = [[Effect::draw(1); 4]; 10];
    let mut vessel = 0;
    while vessel < 10 {
        let mut tier = 0;
        while tier < 4 {
            effects[vessel][tier] = effect(Vessel::ALL[vessel], tier);
            tier += 1;
        }
        vessel += 1;
    }
    effects
};

const LOOT: &[Effect] = &[
    Effect::draw(1),
    Effect::DiscardForPlayers {
        who: PlayerRel::You,
        count: 1,
    },
];

pub(super) const fn of(vessel: Vessel, tier: usize) -> &'static [Effect] {
    if matches!(vessel, Vessel::Lens) {
        LOOT
    } else {
        std::slice::from_ref(&EFFECTS[vessel as usize][tier])
    }
}
