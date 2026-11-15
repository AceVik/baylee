#[allow(clippy::wildcard_imports)]
use super::*;
use domain::v2::{run::Outcome, shop::Delivery};

mod growth;
mod life;

pub(super) fn card(id: baylee_core::ids::CardIndex, lang: Lang) -> String {
    baylee_cards::by_index(id).map_or_else(
        || Phrase::EchoesUnknownCard.text(lang).to_string(),
        |card| card.name().to_string(),
    )
}

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
        Error::Unavailable | Error::Busy => Phrase::ChallengeFailed,
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

pub(super) fn goods(goods: Goods, lang: Lang) -> String {
    if let Goods::Relic(definition) = goods {
        format!("{} · {}", name(goods, lang), rarity(definition.rarity))
    } else {
        name(goods, lang)
    }
}

pub(super) const fn rarity(rarity: Rarity) -> &'static str {
    match rarity {
        Rarity::Common => "Common",
        Rarity::Uncommon => "Uncommon",
        Rarity::Rare => "Rare",
        Rarity::MythicRare => "Mythic Rare",
    }
}

pub(super) fn name(goods: Goods, lang: Lang) -> String {
    match goods {
        Goods::Card(id) => card(id, lang),
        Goods::Relic(Definition { family, .. }) => match family {
            Family::Heartwood => Phrase::EchoesHeartwood,
            Family::Satchel => Phrase::EchoesSatchel,
            Family::Compass => Phrase::EchoesCompass,
            Family::Grove => Phrase::EchoesGrove,
            Family::SpringChalice => Phrase::EchoesChalice,
            Family::Hearthstone => Phrase::EchoesHearth,
            Family::BloodAmber => Phrase::EchoesAmber,
            Family::LifeCoil => Phrase::EchoesCoil,
            Family::SeedCradle => Phrase::EchoesCradle,
            Family::SwarmBanner => Phrase::EchoesBanner,
            Family::HarvestWheel => Phrase::EchoesHarvest,
            Family::RootReservoir => Phrase::EchoesRoots,
            Family::EmberLantern => Phrase::EchoesLantern,
            Family::TideglassLens => Phrase::EchoesLens,
            Family::GraveBell => Phrase::EchoesBell,
            Family::StormSpindle => Phrase::EchoesSpindle,
            Family::DawnAegis => Phrase::EchoesAegis,
            Family::GildedAcorn => Phrase::EchoesAcorn,
        }
        .text(lang)
        .to_owned(),
        Goods::Potion(Potion::Healing) => Phrase::EchoesHealing.text(lang).to_string(),
        Goods::Potion(Potion::Mana) => Phrase::EchoesMana.text(lang).to_string(),
        Goods::Mythic => Phrase::EchoesMythic.text(lang).to_string(),
    }
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
                | Family::GildedAcorn => run_effect(definition, lang),
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
        | Family::GildedAcorn => run_effect(definition, lang),
    }
}

/// Generation-five native sentences; the redesigned shop adds localized detail rows.
pub(super) fn run_effect(definition: Definition, lang: Lang) -> String {
    let Some(token) = baylee_cards::tokens::tower::run::definition(definition.id()) else {
        return String::new();
    };
    let mut lines = Vec::new();
    if definition.family == Family::GildedAcorn {
        lines.push(Phrase::EchoesAcornEffect.fill(lang, &[&definition.rarity.tier().to_string()]));
        if definition.rarity == Rarity::MythicRare {
            lines.push(Phrase::EchoesAcornMythic.text(lang).to_owned());
        }
    }
    for index in 0..token.abilities.len() {
        if let Some(text) = baylee_cards::tokens::tower::run::ability_text(
            token,
            u32::try_from(index).expect("bounded native abilities"),
        ) {
            lines.push(text.to_owned());
        }
    }
    lines.join("\n")
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
    match action {
        Action::Back | Action::Cancel => glyph::ARROW_LEFT,
        Action::Refresh | Action::Retry | Action::Reroll | Action::ResetDeck => glyph::REFRESH,
        Action::Card(_, true) | Action::Ingredient(_) | Action::Material(_) | Action::Mythic(_) => {
            glyph::PLUS
        }
        Action::Card(_, false) | Action::Sell | Action::Abandon => glyph::MINUS,
        Action::Play(_) | Action::Resume => glyph::ARROW_RIGHT,
        Action::Accept
        | Action::Buy
        | Action::SaveDeck
        | Action::Fuse
        | Action::Reward(_)
        | Action::ClaimReward => glyph::CHECK,
        Action::Overview | Action::Read(Read::Runs(_) | Read::Run(_)) | Action::Tiers(_) => {
            glyph::VIEW_ROWS
        }
        Action::Page(Page::Shop) | Action::GoShop => glyph::VIEW_GRID,
        Action::Page(Page::Equipment) | Action::Move | Action::Place(_) => glyph::GEAR,
        Action::Page(Page::Deck) => glyph::LIBRARY,
        Action::Offer(_) | Action::Select(_) | Action::InspectCard(_) => glyph::EYE,
        Action::Pin(_) | Action::Freeze => glyph::CLOCK,
        Action::Withdraw => glyph::EYE_SLASH,
        _ => glyph::COMPASS,
    }
}

pub(super) fn selected(action: Action, ui: &Ui) -> bool {
    match action {
        Action::Page(page) => page == ui.page,
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
