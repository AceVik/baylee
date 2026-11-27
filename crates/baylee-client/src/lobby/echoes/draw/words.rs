#[allow(clippy::wildcard_imports)]
use super::*;
pub(super) use crate::towerui::words::{card, goods, name, rarity, run_effect};
use domain::v2::{run::Outcome, shop::Delivery};

mod growth;
mod life;

/// Crossed swords (`fa-solid-900.ttf` U+F6DE): enter next Tower duel.
pub(super) const GLYPH_DUEL: char = '\u{f6de}';
/// Snowflake (`fa-solid-900.ttf` U+F2DC): freeze or unfreeze the shop shelf.
pub(super) const GLYPH_SNOWFLAKE: char = '\u{f2dc}';
/// Dice (`fa-solid-900.ttf` U+F522): reroll shop offers.
pub(super) const GLYPH_DICE: char = '\u{f522}';
/// Alphabetic sort (`fa-solid-900.ttf` U+F15D): sort inventory by name (A–Z).
pub(super) const GLYPH_SORT_ALPHA: char = '\u{f15d}';
/// Cubes (`fa-solid-900.ttf` U+F1B3): sort inventory by item family.
pub(super) const GLYPH_CUBES: char = '\u{f1b3}';
/// Open folder (`fa-solid-900.ttf` U+F07C): group inventory items.
pub(super) const GLYPH_FOLDER: char = '\u{f07c}';
/// Swap arrows (`fa-solid-900.ttf` U+F0EC): equip or store item.
pub(super) const GLYPH_SWAP: char = '\u{f0ec}';
/// Flag (`fa-solid-900.ttf` U+F024): retire / abandon expedition.
pub(super) const GLYPH_FLAG: char = '\u{f024}';

#[cfg(test)]
pub(super) const TOWER_UI_GLYPHS: [char; 8] = [
    GLYPH_DUEL,
    GLYPH_SNOWFLAKE,
    GLYPH_DICE,
    GLYPH_SORT_ALPHA,
    GLYPH_CUBES,
    GLYPH_FOLDER,
    GLYPH_SWAP,
    GLYPH_FLAG,
];

pub(super) fn created(id: Uuid, lang: Lang) -> String {
    id.get_timestamp()
        .and_then(|at| i64::try_from(at.to_unix().0).ok())
        .map_or_else(
            || "—".into(),
            |at| {
                format!(
                    "{} UTC",
                    baylee_client_core::challenges::presentation::date(at, lang)
                )
            },
        )
}

pub(super) const fn phase(phase: api::Phase) -> Phrase {
    match phase {
        api::Phase::Shop => Phrase::EchoesShopReady,
        api::Phase::Reserved | api::Phase::Playing => Phrase::TowerPlaying,
        api::Phase::Verifying => Phrase::EchoesVerifying,
        api::Phase::Reward => Phrase::EchoesRewardReady,
        api::Phase::Quarantined => Phrase::EchoesQuarantined,
        api::Phase::Conquered => Phrase::TowerConquered,
        api::Phase::Exhausted => Phrase::TowerExhausted,
        api::Phase::Abandoned => Phrase::TowerAbandoned,
        api::Phase::Interrupted => Phrase::EchoesInterrupted,
    }
}

pub(super) const fn outcome(result: Outcome) -> Phrase {
    match result {
        Outcome::Victory => Phrase::EchoesWon,
        Outcome::Defeat => Phrase::EchoesLost,
        Outcome::Draw => Phrase::EchoesDraw,
    }
}

pub(super) const fn failure(error: baylee_client_core::echoes::Error) -> Phrase {
    use baylee_client_core::echoes::Error;
    match error {
        Error::Rejected => Phrase::EchoesRejected,
        Error::Unavailable | Error::Busy => Phrase::EchoesUnreachable,
        Error::Conflict => Phrase::EchoesConflict,
        Error::TermsRequired => Phrase::EchoesTerms,
        Error::UpdateRequired => Phrase::EchoesUpdate,
    }
}

pub(super) fn slot(slot: Slot, lang: Lang) -> String {
    let (phrase, index) = match slot {
        Slot::Active(index) => (Phrase::EchoesActiveSlot, index),
        Slot::Reserve(index) => (Phrase::EchoesReserveSlot, index),
        Slot::Inventory => return Phrase::EchoesInventory.text(lang).to_owned(),
    };
    phrase.fill(lang, &[&(index + 1).to_string()])
}

pub(super) fn effect(goods: Goods, lang: Lang) -> String {
    match goods {
        Goods::Card(_) => Phrase::EchoesCardEffect.text(lang).into(),
        Goods::Potion(Potion::Healing) => Phrase::EchoesHealingEffect.text(lang).into(),
        Goods::Potion(Potion::Mana) => Phrase::EchoesManaEffect.text(lang).into(),
        Goods::Mythic => Phrase::EchoesMythicEffect.text(lang).into(),
        Goods::Relic(definition) => {
            let stats = domain::v2::catalog::attributes(&[definition]).expect("one admitted relic");
            match definition.family {
                Family::Heartwood => {
                    Phrase::EchoesLifeEffect.fill(lang, &[&(stats.life - 20).to_string()])
                }
                Family::Satchel if stats.cards == 7 => Phrase::EchoesHandLimitEffect
                    .fill(lang, &[&(stats.maximum_hand - 7).to_string()]),
                Family::Satchel => Phrase::EchoesCardsEffect.fill(
                    lang,
                    &[
                        &(stats.cards - 7).to_string(),
                        &(stats.maximum_hand - 7).to_string(),
                    ],
                ),
                Family::Compass => Phrase::EchoesMulliganEffect
                    .fill(lang, &[&(stats.free_mulligans - 1).to_string()]),
                Family::Grove => Phrase::EchoesGroveEffect.fill_count(
                    lang,
                    usize::from(definition.rarity.tier()),
                    &[],
                ),
                Family::SpringChalice
                | Family::Hearthstone
                | Family::BloodAmber
                | Family::LifeCoil => life::effect(definition, lang),
                Family::SeedCradle
                | Family::SwarmBanner
                | Family::HarvestWheel
                | Family::RootReservoir => growth::effect(definition, lang),
                Family::EmberLantern
                | Family::TideglassLens
                | Family::GraveBell
                | Family::StormSpindle
                | Family::DawnAegis
                | Family::GildedAcorn
                | Family::Runic(_) => run_effect(definition, lang),
            }
        }
    }
}

pub(super) fn tier(definition: Definition, lang: Lang) -> String {
    match definition.family {
        Family::Grove => {
            Phrase::EchoesGroveTier.fill_count(lang, usize::from(definition.rarity.tier()), &[])
        }
        Family::Heartwood | Family::Satchel | Family::Compass => {
            effect(Goods::Relic(definition), lang)
        }
        Family::SpringChalice | Family::Hearthstone | Family::BloodAmber | Family::LifeCoil => {
            life::tier(definition, lang)
        }
        Family::SeedCradle | Family::SwarmBanner | Family::HarvestWheel | Family::RootReservoir => {
            growth::tier(definition, lang)
        }
        Family::EmberLantern
        | Family::TideglassLens
        | Family::GraveBell
        | Family::StormSpindle
        | Family::DawnAegis
        | Family::GildedAcorn
        | Family::Runic(_) => run_effect(definition, lang),
    }
}

pub(super) fn command(command: &Command, ui: &Ui, lang: Lang) -> String {
    let Some(run) = ui.model.run() else {
        return Phrase::EchoesConflict.text(lang).into();
    };
    let item = |id| item_name(run, id, lang);
    let recipe = |recipe: &domain::v2::economy::Recipe| {
        let mut inputs: Vec<_> = recipe.relics.iter().map(|id| item(*id)).collect();
        if recipe.mythic.is_some() {
            inputs.push(Phrase::EchoesMythic.text(lang).to_string());
        }
        Phrase::EchoesConfirmFuse.fill(
            lang,
            &[&inputs.join(" + "), &slot(recipe.destination, lang)],
        )
    };
    command_text(command, ui, run, lang, item, recipe)
}

fn item_name(run: &api::Run, id: u32, lang: Lang) -> String {
    run.economy
        .inventory
        .relics
        .iter()
        .find(|relic| relic.id == id)
        .map_or_else(
            || Phrase::EchoesMythic.text(lang).to_string(),
            |relic| {
                format!(
                    "{} ({})",
                    goods(Goods::Relic(relic.definition), lang),
                    slot(relic.slot, lang)
                )
            },
        )
}

fn command_text(
    command: &Command,
    ui: &Ui,
    run: &api::Run,
    lang: Lang,
    item: impl Fn(u32) -> String,
    recipe: impl Fn(&domain::v2::economy::Recipe) -> String,
) -> String {
    match command {
        Command::Buy { offer, delivery } => {
            let Some(offer) = run.shop.offers.iter().find(|entry| entry.id == *offer) else {
                return Phrase::EchoesConflict.text(lang).into();
            };
            let price = ui
                .model
                .publication()
                .and_then(|cohort| offer.goods.price(&cohort.catalog).ok())
                .unwrap_or(0);
            let destination = match delivery {
                Delivery::Inventory => Phrase::EchoesInventory.text(lang).to_string(),
                Delivery::Slot(to) => slot(*to, lang),
                Delivery::Fuse(selected) => recipe(selected),
            };
            Phrase::EchoesConfirmBuy.fill(
                lang,
                &[&goods(offer.goods, lang), &price.to_string(), &destination],
            )
        }
        Command::Sell { item: id } => Phrase::EchoesConfirmSell.fill(
            lang,
            &[
                &item(*id),
                &baylee_client_core::echoes::draft::refund(run, *id)
                    .unwrap_or(0)
                    .to_string(),
            ],
        ),
        Command::Move { item: id, slot: to } => {
            Phrase::EchoesConfirmMove.fill(lang, &[&item(*id), &slot(*to, lang)])
        }
        Command::Fuse { recipe: selected } => recipe(selected),
        Command::Edit { deck } => Phrase::EchoesConfirmDeck.fill(
            lang,
            &[&deck
                .iter()
                .map(|entry| u16::from(entry.copies))
                .sum::<u16>()
                .to_string()],
        ),
        Command::Refit { .. } => Phrase::EchoesConfirmDeck.fill(lang, &["40"]),
        Command::Claim { cards } => {
            Phrase::EchoesConfirmReward.fill(lang, &[&cards.len().to_string()])
        }
        Command::Pin { offer, pinned } => Phrase::EchoesConfirmPin.fill(
            lang,
            &[
                &run.shop
                    .offers
                    .iter()
                    .find(|entry| entry.id == *offer)
                    .map_or_else(|| "—".into(), |offer| goods(offer.goods, lang)),
                if *pinned {
                    Phrase::EchoesPin
                } else {
                    Phrase::EchoesUnpin
                }
                .text(lang),
            ],
        ),
        Command::Refresh {} => Phrase::EchoesConfirmRefresh
            .fill(lang, &[&run.shop.reroll_price().unwrap_or(0).to_string()]),
        Command::Freeze { frozen } => if *frozen {
            Phrase::EchoesFreezeConfirm
        } else {
            Phrase::EchoesThawConfirm
        }
        .text(lang)
        .to_owned(),
        Command::ChooseMythic { slot, .. } => {
            Phrase::EchoesConfirmMythic.fill(lang, &[&(*slot + 1).to_string()])
        }
        Command::SellCard { card: id } => Phrase::EchoesConfirmSellCard.fill(
            lang,
            &[
                &card(*id, lang),
                &ui.model
                    .publication()
                    .and_then(|cohort| {
                        baylee_client_core::echoes::draft::sellable_card_gold(run, cohort, *id)
                    })
                    .unwrap_or(0)
                    .to_string(),
            ],
        ),
        Command::Abandon {} => Phrase::TowerRetireConfirm.text(lang).to_string(),
    }
}

pub(super) fn destination(delivery: &Delivery, lang: Lang) -> String {
    let name = match delivery {
        Delivery::Inventory => Phrase::EchoesInventory.text(lang).to_owned(),
        Delivery::Slot(place) => slot(*place, lang),
        Delivery::Fuse(recipe) => {
            Phrase::EchoesFusionDestination.fill(lang, &[&slot(recipe.destination, lang)])
        }
    };
    Phrase::EchoesDestination.fill(lang, &[&name])
}

pub(super) const fn icon(action: Action) -> char {
    use baylee_client_core::echoes::workshop::Order;
    match action {
        Action::Back | Action::Cancel => glyph::ARROW_LEFT,
        Action::Refresh | Action::Retry | Action::ResetDeck => glyph::REFRESH,
        Action::Reroll => GLYPH_DICE,
        Action::Card(_, true) | Action::Ingredient(_) | Action::Material(_) | Action::Mythic(_) => {
            glyph::PLUS
        }
        #[cfg(test)]
        Action::ZoneCard(_, _, true) => glyph::PLUS,
        Action::Card(_, false) => glyph::MINUS,
        #[cfg(test)]
        Action::ZoneCard(_, _, false) => glyph::MINUS,
        Action::Sell
        | Action::SellCard(_)
        | Action::ToggleSpareCards
        | Action::ToggleRewardGold => glyph::COINS,
        Action::Abandon => GLYPH_FLAG,
        Action::Badge(_)
        | Action::Page(Page::Collection | Page::Ranking)
        | Action::Read(Read::Ranking(..)) => glyph::TROPHY,
        Action::ClosePreview => glyph::CLOSE,
        Action::Play(_) => GLYPH_DUEL,
        Action::Resume => glyph::ARROW_RIGHT,
        Action::Accept | Action::Buy | Action::SaveDeck | Action::Fuse | Action::Reward(_) => {
            glyph::CHECK
        }
        Action::ClaimReward | Action::Upgrade(..) => glyph::CROWN,
        Action::QuickEquip(_) => GLYPH_SWAP,
        Action::InventoryOrder(order) => match order {
            Order::Newest => glyph::CLOCK,
            Order::Name => GLYPH_SORT_ALPHA,
            Order::Value => glyph::COINS,
            Order::Family => GLYPH_CUBES,
            Order::Rarity => glyph::DIAMOND,
        },
        Action::InventoryGroups(_) => GLYPH_FOLDER,
        Action::Read(Read::Runs(_) | Read::Run(_)) | Action::Tiers(_) => glyph::VIEW_ROWS,
        Action::Page(Page::Shop) | Action::GoShop => glyph::VIEW_GRID,
        Action::Page(Page::Equipment) | Action::Move | Action::Place(_) => glyph::GEAR,
        Action::InspectStarter(..) | Action::Page(Page::Deck) | Action::ToggleRewardDeck => {
            glyph::LIBRARY
        }
        Action::Listing(true) | Action::Offer(_) | Action::Select(_) | Action::InspectCard(_) => {
            glyph::EYE
        }
        Action::Pin(_) => glyph::CLOCK,
        Action::Freeze => GLYPH_SNOWFLAKE,
        Action::Listing(false) | Action::Withdraw => glyph::EYE_SLASH,
        _ => glyph::COMPASS,
    }
}

pub(super) fn selected(action: Action, ui: &Ui) -> bool {
    match action {
        Action::Page(page) => page == ui.page,
        Action::Read(Read::Ranking(cohort, lane)) => ui.ranking == Some((cohort, lane)),
        Action::Badge(kind) => kind == ui.collection.selected,
        Action::InventoryOrder(order) => order == ui.bag.order,
        Action::InventoryGroups(_) => ui.bag.grouped,
        Action::ToggleSpareCards => ui.spare_cards_open,
        Action::ToggleRewardGold => ui.reward_gold_open,
        Action::ToggleRewardDeck => ui.reward_deck_open,
        Action::Offer(id) => ui.choices.offer == Some(id),
        Action::Select(id) => ui.choices.item == Some(id),
        Action::InspectCard(id) => ui.preview == Some(id),
        Action::Place(slot) => ui.choices.slot == slot,
        Action::Reward(card) => ui.choices.rewards.contains(&card),
        Action::Freeze => ui.model.run().is_some_and(|run| run.shop.frozen),
        Action::Ingredient(id) => ui.choices.ingredients.contains(&id),
        Action::Material(id) => ui.choices.material == Some(id),
        Action::Tiers(family) => ui.tiers == Some(family),
        _ => false,
    }
}
