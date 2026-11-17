use super::{Action, Page, Read, Slot, Ui};
use crate::shellkit::focus::{Stop, TabOrder};

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "echoes",
    stops: &[
        "back",
        "overview",
        "refresh",
        "retry",
        "play",
        "hosts",
        "tabs",
        "badges",
        "ranking-lane",
        "listing",
        "shop-return",
        "reroll",
        "freeze",
        "rewards",
        "claim-reward",
        "offers",
        "pins",
        "slots",
        "ingredients",
        "materials",
        "buy",
        "tiers",
        "items",
        "inventory-page",
        "inventory-order",
        "inventory-groups",
        "move",
        "move-slot",
        "sell",
        "spare-toggle",
        "sell-card",
        "fuse",
        "card-preview",
        "close-preview",
        "deck-zone",
        "deck-page",
        "commander",
        "remove",
        "add",
        "save",
        "reset",
        "mythic",
        "abandon",
        "runs",
        "run-page",
        "starters",
        "starter-preview",
        "cohort-page",
        "withdraw",
        "baylee",
    ],
    modal: false,
};

pub(crate) const CONFIRM: TabOrder = TabOrder {
    name: "echoes-confirm",
    stops: &["cancel", "confirm"],
    modal: true,
};
pub(crate) const INSPECT: TabOrder = TabOrder {
    name: "echoes-inspect",
    stops: &["close-preview"],
    modal: true,
};
pub(crate) const REWARD: TabOrder = TabOrder {
    name: "echoes-reward",
    stops: &["reward-gold", "reward-deck", "rewards", "claim-reward"],
    modal: true,
};

fn index(ui: &Ui, id: u32, offer: bool) -> usize {
    ui.model
        .run()
        .and_then(|run| {
            if offer {
                run.shop.offers.iter().position(|entry| entry.id == id)
            } else {
                run.economy
                    .inventory
                    .relics
                    .iter()
                    .map(|item| item.id)
                    .chain(run.economy.inventory.materials.iter().map(|item| item.id))
                    .position(|item| item == id)
            }
        })
        .unwrap_or(0)
}

fn reward_index(ui: &Ui, card: baylee_core::ids::CardIndex) -> usize {
    ui.model
        .run()
        .and_then(|run| run.reward.as_ref())
        .and_then(|reward| reward.cards.iter().position(|id| *id == card))
        .unwrap_or(0)
}

fn sell_card_index(ui: &Ui, card: baylee_core::ids::CardIndex) -> usize {
    ui.model
        .run()
        .zip(ui.model.publication())
        .and_then(|(run, cohort)| {
            baylee_client_core::echoes::draft::sellable_cards(run, cohort)
                .into_iter()
                .position(|(entry, _)| entry == card)
        })
        .unwrap_or(0)
}

pub(super) fn stop(action: Action, ui: &Ui) -> Stop {
    if ui.rewarding() {
        match action {
            Action::Reward(card) => {
                return Stop::item(
                    REWARD.name,
                    "rewards",
                    u8::try_from(reward_index(ui, card)).unwrap_or(0),
                );
            }
            Action::ClaimReward => return Stop::new(REWARD.name, "claim-reward"),
            Action::ToggleRewardGold => return Stop::new(REWARD.name, "reward-gold"),
            Action::ToggleRewardDeck => return Stop::new(REWARD.name, "reward-deck"),
            _ => {}
        }
    }
    stop_regular(action, ui)
}

#[allow(clippy::too_many_lines)] // One exhaustive action-to-focus mapping, without hidden fallbacks.
fn stop_regular(action: Action, ui: &Ui) -> Stop {
    let (key, at) = match action {
        Action::Accept => return Stop::new(CONFIRM.name, "confirm"),
        Action::Cancel => return Stop::new(CONFIRM.name, "cancel"),
        Action::Open | Action::Back => ("back", 0),
        Action::Refresh => ("refresh", 0),
        Action::Retry => ("retry", 0),
        Action::GoShop => ("shop-return", 0),
        Action::Withdraw => ("withdraw", 0),
        Action::Resume => ("play", 0),
        Action::Play(id) => (
            "play",
            ui.model
                .hosts()
                .iter()
                .position(|host| host.id == id)
                .unwrap_or(0),
        ),
        Action::Page(page) => ("tabs", page_index(page)),
        Action::Badge(kind) => ("badges", badge_index(kind)),
        Action::Listing(_) => ("listing", 0),
        Action::Read(Read::Ranking(cohort, lane)) => {
            ("ranking-lane", ranking_index(ui, cohort, lane))
        }
        Action::Offer(id) | Action::Purchase(id) | Action::PurchaseEquipped(id) => {
            ("offers", index(ui, id, true))
        }
        Action::QuickEquip(id) => ("move", index(ui, id, false)),
        Action::Transfer(_, slot) => (
            "move-slot",
            match slot {
                Slot::Active(at) | Slot::Reserve(at) => usize::from(at),
                Slot::Inventory => 6,
            },
        ),
        Action::Upgrade(id, _) => ("fuse", index(ui, id, false)),
        Action::InventoryPage(page) => ("inventory-page", page),
        Action::InventoryOrder(order) => (
            "inventory-order",
            match order {
                baylee_client_core::echoes::workshop::Order::Newest => 0,
                baylee_client_core::echoes::workshop::Order::Name => 1,
                baylee_client_core::echoes::workshop::Order::Value => 2,
                baylee_client_core::echoes::workshop::Order::Family => 3,
                baylee_client_core::echoes::workshop::Order::Rarity => 4,
            },
        ),
        Action::InventoryGroups(_) => ("inventory-groups", 0),
        Action::ToggleSpareCards => ("spare-toggle", 0),
        Action::SellCard(card) => ("sell-card", sell_card_index(ui, card)),
        Action::ToggleRewardGold => ("reward-gold", 0),
        Action::ToggleRewardDeck => ("reward-deck", 0),
        Action::Pin(id) => ("pins", index(ui, id, true)),
        Action::Select(id) => ("items", index(ui, id, false)),
        Action::Ingredient(id) => ("ingredients", index(ui, id, false)),
        Action::Material(id) => ("materials", index(ui, id, false)),
        Action::Place(slot) => (
            "slots",
            match slot {
                Slot::Active(i) => usize::from(i),
                Slot::Reserve(i) => usize::from(i) + 3,
                Slot::Inventory => 9,
            },
        ),
        Action::InspectStarter(_, index) => ("starter-preview", usize::from(index)),
        Action::ClosePreview if ui.starter_preview.is_some() || ui.preview.is_some() => {
            return Stop::new(INSPECT.name, "close-preview");
        }
        Action::ClosePreview => ("close-preview", 0),
        Action::Card(card, _) | Action::InspectCard(card) => {
            (card_key(action), card_index(ui, card))
        }
        #[cfg(test)]
        Action::ZoneCard(card, _, _) => (card_key(action), card_index(ui, card)),
        Action::Buy => ("buy", 0),
        Action::Tiers(_) => ("tiers", 0),
        Action::Move => ("move", 0),
        Action::Sell => ("sell", 0),
        Action::Fuse => ("fuse", 0),
        Action::Reroll => ("reroll", 0),
        Action::Freeze => ("freeze", 0),
        Action::ClaimReward => ("claim-reward", 0),
        Action::Reward(card) => ("rewards", reward_index(ui, card)),
        Action::Abandon => ("abandon", 0),
        Action::Mythic(slot) => ("mythic", usize::from(slot)),
        Action::SaveDeck => ("save", 0),
        Action::ResetDeck => ("reset", 0),
        Action::Read(Read::Run(id)) => (
            "runs",
            ui.model
                .runs()
                .iter()
                .position(|run| run.id == id)
                .unwrap_or(0),
        ),
        Action::Read(Read::Runs(before)) => ("run-page", usize::from(before.is_some())),
        Action::Read(Read::Hosts(_)) => ("hosts", 0),
        Action::Read(_) => ("cohort-page", 0),
        Action::Starter(id, starter) => ("starters", starter_index(ui, id, starter)),
    };
    Stop::item(ORDER.name, key, u8::try_from(at).unwrap_or(u8::MAX))
}

fn ranking_index(ui: &Ui, cohort: uuid::Uuid, lane: super::api::ranking::Lane) -> usize {
    let at = if ui.ranking.is_some_and(|(shown, _)| shown == cohort) {
        0
    } else {
        1 + ui
            .model
            .cohorts()
            .iter()
            .position(|c| c.id == cohort)
            .unwrap_or(8)
    };
    at * 2 + usize::from(lane == super::api::ranking::Lane::House)
}

fn starter_index(ui: &Ui, id: uuid::Uuid, starter: u8) -> usize {
    ui.model
        .cohorts()
        .iter()
        .position(|cohort| cohort.id == id)
        .unwrap_or(0)
        * 8
        + usize::from(starter)
}

fn card_key(action: Action) -> &'static str {
    match action {
        Action::Card(_, true) => "add",
        #[cfg(test)]
        Action::ZoneCard(_, _, true) => "add",
        Action::Card(_, false) => "remove",
        #[cfg(test)]
        Action::ZoneCard(_, _, false) => "remove",
        _ => "card-preview",
    }
}

fn card_index(ui: &Ui, card: baylee_core::ids::CardIndex) -> usize {
    ui.model
        .publication()
        .and_then(|c| c.catalog.cards.iter().position(|entry| entry.card == card))
        .unwrap_or(0)
}

const fn page_index(page: Page) -> usize {
    match page {
        Page::Overview => 3,
        Page::Shop => 0,
        Page::Equipment => 1,
        Page::Deck => 2,
        Page::Collection => 4,
        Page::Ranking => 5,
    }
}

const fn badge_index(kind: baylee_client_core::echoes::awards_api::BadgeKind) -> usize {
    use baylee_client_core::echoes::awards_api::BadgeKind;
    match kind {
        BadgeKind::Victories => 0,
        BadgeKind::Ascents => 1,
        BadgeKind::Unbroken => 2,
    }
}
