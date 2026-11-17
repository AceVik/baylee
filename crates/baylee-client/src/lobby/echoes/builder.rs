//! Adapter between the shared editor and a revision-bound Tower refit. No deck
//! library writes, no independent filters, layout, drag code or card previews.
use super::{Command, LobbyState, Page, Phrase, api, domain};
use crate::lobby::{BuildPress, Hub};
use baylee_client_core::deckbuilder::{
    DeckBuilder, Zone,
    limited::{Commander, Holding},
};
use baylee_core::{deckrow::PrintChoice, ids::CardIndex};
use domain::cards::{Formation, Stack};
use std::collections::BTreeMap;
use uuid::Uuid;

enum Sent {
    Refit(Command),
    Sell {
        card: CardIndex,
        card_name: String,
        gold: u16,
        prev_copies: u8,
    },
}

pub(super) struct Session {
    run: Uuid,
    revision: u64,
    sent: Option<Sent>,
    uncertain_seen: bool,
}

/// Compact `(revision, busy, gold)` signature for retained `buildui` keys.
#[must_use]
pub(crate) fn sell_signature(state: &LobbyState) -> (u64, bool, u32) {
    if !state.lobby.scoped_builder() {
        return (0, false, 0);
    }
    let busy = state.echoes.model.busy()
        || state.echoes.model.uncertain()
        || state
            .echoes
            .editor
            .as_ref()
            .is_some_and(|edit| edit.sent.is_some());
    let (rev, gold) = state
        .echoes
        .model
        .run()
        .map_or((0, 0), |run| (run.revision, u32::from(run.economy.gold)));
    (rev, busy, gold)
}

/// Authoritative + working-draft sale eligibility for pool `slot`.
#[must_use]
pub(crate) fn sell_status(state: &LobbyState, slot: usize) -> crate::buildui::TowerSellStatus {
    use crate::buildui::TowerSellStatus;

    if !state.lobby.scoped_builder() {
        return TowerSellStatus::Hidden;
    }
    let deck = state.lobby.builder();
    let Some(card) = deck.card(slot) else {
        return TowerSellStatus::Hidden;
    };
    if card.basic_land
        || deck.count_of(slot, Zone::Main) > 0
        || deck.count_of(slot, Zone::Side) > 0
        || deck.is_commander(slot)
    {
        return TowerSellStatus::Hidden;
    }
    let Some(run) = state.echoes.model.run() else {
        return TowerSellStatus::Hidden;
    };
    let Some(cohort) = state.echoes.model.publication() else {
        return TowerSellStatus::Hidden;
    };
    let card_id = CardIndex::new(card.index);
    if let Some(gold) = baylee_client_core::echoes::draft::sellable_card_gold(run, cohort, card_id)
    {
        let busy = state.echoes.model.busy()
            || state.echoes.model.uncertain()
            || state
                .echoes
                .editor
                .as_ref()
                .is_some_and(|edit| edit.sent.is_some());
        return TowerSellStatus::Sellable { gold, busy };
    }
    if deck.dirty()
        && run.phase == api::Phase::Shop
        && domain::v2::generation::is_run(run.shop.generator)
        && run.cards.copies(card_id) > 0
        && let Some(gold) = cohort.catalog.card_sale_gold(card_id)
    {
        return TowerSellStatus::SaveRefitFirst { gold };
    }
    TowerSellStatus::Hidden
}

pub(super) fn open(state: &mut LobbyState) -> bool {
    let Some(run) = state.echoes.model.run() else {
        return false;
    };
    let Some(formation) = &run.cards.formation else {
        return false;
    };
    if run.phase != api::Phase::Shop || state.echoes.model.busy() || state.echoes.model.uncertain()
    {
        return false;
    }
    let Some(cohort) = state.echoes.model.publication() else {
        return false;
    };
    let mut stock = BTreeMap::new();
    let mut pool = Vec::new();
    for entry in &cohort.catalog.cards {
        if !entry.basic_land && run.cards.copies(entry.card) == 0 {
            continue;
        }
        let Some(definition) = baylee_cards::by_index(entry.card) else {
            return false;
        };
        let mut card = state
            .lobby
            .builder()
            .card_by_index(entry.card.get())
            .cloned()
            .unwrap_or_else(|| {
                crate::lobby::offline::pool_row(&baylee_cards::pool::row_without_partners(
                    definition,
                ))
            });
        let print = entry.run.and_then(|facts| facts.presentation).map_or_else(
            PrintChoice::default,
            |presentation| PrintChoice {
                scryfall_id: Some(presentation.printing.to_string()),
                finish: Some(presentation.finish),
                ..PrintChoice::default()
            },
        );
        if let Some(id) = &print.scryfall_id {
            card.scryfall_id.clone_from(id);
        }
        stock.insert(
            entry.card.get(),
            Holding {
                copies: if entry.basic_land { u16::MAX } else { 1 },
                print,
            },
        );
        pool.push(card);
    }
    let rows = |cards: &[Stack]| -> Vec<String> {
        cards
            .iter()
            .filter_map(|stack| {
                baylee_cards::by_index(stack.card)
                    .map(|card| format!("{} {}", stack.copies, card.name()))
            })
            .collect()
    };
    let Some(leader) = baylee_cards::by_index(formation.commander) else {
        return false;
    };
    let mut main = rows(&run.cards.deck);
    main.push(format!("1 {}", leader.name()));
    let mut editor = DeckBuilder::new();
    editor.set_pool(pool, false);
    editor.localize_types(state.lobby.lang());
    editor.load(
        &format!("tower-{}", run.id),
        Phrase::EchoesDeck.text(state.lobby.lang()),
        &main,
        &rows(&formation.sideboard),
        &[leader.name().to_string()],
    );
    editor.limit_to(Commander {
        total: 40,
        side: 5,
        stock,
    });
    state.echoes.editor = Some(Session {
        run: run.id,
        revision: run.revision,
        sent: None,
        uncertain_seen: false,
    });
    state.lobby.open_scoped_builder(editor);
    state.build = crate::buildui::BuildUi::default();
    true
}

fn stacks(cards: Vec<CardIndex>) -> Vec<Stack> {
    let mut counts = BTreeMap::<CardIndex, u8>::new();
    for card in cards {
        *counts.entry(card).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(card, copies)| Stack { card, copies })
        .collect()
}

fn composition(state: &LobbyState) -> Option<Command> {
    if !state.lobby.scoped_builder() || !state.lobby.builder().saveable() {
        return None;
    }
    let (main, side, leaders) = state.lobby.builder().composition();
    let [commander] = leaders.as_slice() else {
        return None;
    };
    Some(Command::Refit {
        deck: stacks(main),
        formation: Formation {
            commander: *commander,
            sideboard: stacks(side),
        },
    })
}

fn save(state: &mut LobbyState) {
    if state.echoes.refit_uncertain() {
        if let Some(work) = state.echoes.model.retry(Uuid::now_v7()) {
            if let Some(edit) = state.echoes.editor.as_mut() {
                edit.uncertain_seen = false;
            }
            state.build.save = crate::buildui::SaveState::Saving;
            super::native::submit(state, work);
        }
        return;
    }
    let Some(edit) = &state.echoes.editor else {
        return;
    };
    let Some(run) = state.echoes.model.run() else {
        return;
    };
    if (run.id, run.revision) != (edit.run, edit.revision) {
        state.lobby.tell_refusal(Phrase::EchoesEditConflict, &[]);
        return;
    }
    let Some(command) = composition(state) else {
        return;
    };
    if !state
        .echoes
        .model
        .publication()
        .is_some_and(|cohort| baylee_client_core::echoes::draft::allowed(run, cohort, &command))
    {
        return;
    }
    if let Some(work) = state.echoes.command(command.clone()) {
        let edit = state.echoes.editor.as_mut().unwrap();
        edit.sent = Some(Sent::Refit(command));
        edit.uncertain_seen = false;
        state.build.save = crate::buildui::SaveState::Saving;
        super::native::submit(state, work);
    }
}

fn confirm_sell(state: &mut LobbyState) {
    let Some(slot) = state.build.confirm_sell.take() else {
        return;
    };
    let Some(edit) = &state.echoes.editor else {
        return;
    };
    let Some(run) = state.echoes.model.run() else {
        return;
    };
    if (run.id, run.revision) != (edit.run, edit.revision) {
        state.lobby.tell_refusal(Phrase::EchoesEditConflict, &[]);
        return;
    }
    let crate::buildui::TowerSellStatus::Sellable { gold, busy: false } = sell_status(state, slot)
    else {
        return;
    };
    let Some(card) = state.lobby.builder().card(slot) else {
        return;
    };
    let card_id = CardIndex::new(card.index);
    let card_name = card.name.clone();
    let prev_copies = run.cards.copies(card_id);
    let command = Command::SellCard { card: card_id };
    if !state
        .echoes
        .model
        .publication()
        .is_some_and(|cohort| baylee_client_core::echoes::draft::allowed(run, cohort, &command))
    {
        return;
    }
    if let Some(work) = state.echoes.command(command) {
        let edit = state.echoes.editor.as_mut().unwrap();
        edit.sent = Some(Sent::Sell {
            card: card_id,
            card_name,
            gold,
            prev_copies,
        });
        edit.uncertain_seen = false;
        super::native::submit(state, work);
    }
}

fn close(state: &mut LobbyState) {
    state.lobby.close_scoped_builder();
    state.echoes.editor = None;
    state.echoes.page = Page::Shop;
    state.hub = Hub::Challenges;
    state.confirm_leave = false;
    state.build = crate::buildui::BuildUi::default();
}

/// Intercepts only the session's persistence/navigation and library-only tools.
/// Every ordinary card edit, search, drag, filter and preview uses the shared code.
pub(in crate::lobby) fn press(state: &mut LobbyState, action: BuildPress) -> bool {
    if !state.lobby.scoped_builder() {
        return false;
    }
    // Reading fresh run state must never swallow local editor gestures. Only
    // leaving an unacknowledged refit or card sale waits for its outcome.
    if state
        .echoes
        .editor
        .as_ref()
        .is_some_and(|edit| edit.sent.is_some())
        && matches!(
            action,
            BuildPress::CloseBuilder | BuildPress::DiscardAndLeave
        )
    {
        return true;
    }
    match action {
        BuildPress::SaveDeck => save(state),
        BuildPress::ConfirmSellTowerCard => confirm_sell(state),
        BuildPress::CancelSellTowerCard => state.build.confirm_sell = None,
        BuildPress::CloseBuilder => {
            if state.lobby.builder().dirty() {
                state.confirm_leave = true;
            } else {
                close(state);
            }
        }
        BuildPress::DiscardAndLeave => close(state),
        BuildPress::KeepEditing => state.confirm_leave = false,
        BuildPress::OpenImport
        | BuildPress::ImportPaste
        | BuildPress::ImportTake
        | BuildPress::Rename
        | BuildPress::FinishRename
        | BuildPress::DeleteBuildDeck
        | BuildPress::ClearDeck
        | BuildPress::AddPartner(_)
        | BuildPress::ToggleHeaderMenu
        | BuildPress::SaveFormatVariant
        | BuildPress::LoadFormatVariant(_)
        | BuildPress::DeleteFormatVariant(_)
        | BuildPress::ToggleCardFavDeck(_)
        | BuildPress::SetFormat(_)
        | BuildPress::ListChoose(_)
        | BuildPress::ListNew
        | BuildPress::ListOpen(_)
        | BuildPress::ListCopyOf(_)
        | BuildPress::FocusBuild(baylee_client_core::deckbuilder::BuildField::Name) => {}
        _ => return false,
    }
    true
}

pub(super) fn pending(state: &LobbyState) -> bool {
    state.lobby.scoped_builder()
        && state.echoes.editor.as_ref().is_some_and(|edit| {
            edit.sent.is_some() && (!state.echoes.model.uncertain() || !edit.uncertain_seen)
        })
}

pub(super) fn reconcile(state: &mut LobbyState, now: f64) {
    if !state.lobby.scoped_builder() || state.echoes.model.busy() {
        return;
    }
    if state.echoes.model.uncertain() {
        if let Some(edit) = state.echoes.editor.as_mut() {
            edit.uncertain_seen = true;
            if matches!(edit.sent, Some(Sent::Refit(_))) {
                state.build.save = crate::buildui::SaveState::Failed;
            }
        }
        return;
    }
    let Some(sent) = state
        .echoes
        .editor
        .as_mut()
        .and_then(|edit| edit.sent.take())
    else {
        return;
    };
    match sent {
        Sent::Refit(sent) => {
            let accepted = state.echoes.model.run().is_some_and(|run| {
                state
                    .echoes
                    .editor
                    .as_ref()
                    .is_some_and(|edit| run.id == edit.run && run.revision > edit.revision)
                    && matches!(
                        &sent,
                        Command::Refit { deck, formation }
                            if run.cards.deck == *deck && run.cards.formation.as_ref() == Some(formation)
                    )
            });
            if accepted {
                let revision = state.echoes.model.run().unwrap().revision;
                state.echoes.editor.as_mut().unwrap().revision = revision;
                if composition(state).as_ref() == Some(&sent) {
                    state.lobby.builder_mut().saved(None);
                }
                state.build.save = crate::buildui::SaveState::Saved {
                    minutes: 0,
                    at: now,
                };
                state.lobby.tell(Phrase::DeckSaved, &[]);
            } else {
                state.build.save = crate::buildui::SaveState::Failed;
                state.lobby.tell_refusal(Phrase::EchoesEditConflict, &[]);
            }
        }
        Sent::Sell {
            card,
            card_name,
            gold,
            prev_copies,
        } => {
            let accepted = state.echoes.model.run().is_some_and(|run| {
                state
                    .echoes
                    .editor
                    .as_ref()
                    .is_some_and(|edit| run.id == edit.run && run.revision > edit.revision)
                    && run.cards.copies(card) < prev_copies
            });
            if accepted {
                let run = state.echoes.model.run().unwrap();
                let revision = run.revision;
                let remaining = u16::from(run.cards.copies(card) > 0);
                state.echoes.editor.as_mut().unwrap().revision = revision;
                state
                    .lobby
                    .builder_mut()
                    .sync_limited_card_copies(card.get(), remaining);
                state
                    .lobby
                    .tell(Phrase::EchoesCardSold, &[&card_name, &gold.to_string()]);
            } else {
                state.lobby.tell_refusal(Phrase::EchoesEditConflict, &[]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildui::TowerSellStatus;
    use baylee_client_core::echoes::Read;
    use baylee_core::color::ColorSet;
    use baylee_protocol::execution::{Identity, Variant};
    use domain::cards::{Card, Catalog, Collection, RunCard};

    #[allow(clippy::too_many_lines)]
    fn seeded_tower_editor() -> (LobbyState, CardIndex) {
        let leader = baylee_cards::decks::by_name("General Tazri").unwrap();
        let plains = baylee_cards::decks::by_name("Plains").unwrap();
        let island = baylee_cards::decks::by_name("Island").unwrap();
        let swamp = baylee_cards::decks::by_name("Swamp").unwrap();
        let mountain = baylee_cards::decks::by_name("Mountain").unwrap();
        let forest = baylee_cards::decks::by_name("Forest").unwrap();
        let ring = baylee_cards::decks::by_name("Sol Ring").unwrap();
        let spare = baylee_cards::decks::by_name("Wrath of God").unwrap();
        let alt_leaders = [
            baylee_cards::decks::by_name("Isamaru, Hound of Konda").unwrap(),
            baylee_cards::decks::by_name("Talrand, Sky Summoner").unwrap(),
            baylee_cards::decks::by_name("Judith, the Scourge Diva").unwrap(),
            baylee_cards::decks::by_name("Alela, Artful Provocateur").unwrap(),
        ];

        let mut cards = vec![
            (plains, baylee_core::color::Color::White),
            (island, baylee_core::color::Color::Blue),
            (swamp, baylee_core::color::Color::Black),
            (mountain, baylee_core::color::Color::Red),
            (forest, baylee_core::color::Color::Green),
        ]
        .into_iter()
        .map(|(card, color)| Card {
            card,
            price: 0,
            basic_land: true,
            run: Some(RunCard {
                presentation: None,
                identity: ColorSet::of(color),
                commander: false,
            }),
        })
        .collect::<Vec<_>>();
        cards.push(Card {
            card: leader,
            price: 4,
            basic_land: false,
            run: Some(RunCard {
                presentation: None,
                identity: ColorSet::ALL,
                commander: true,
            }),
        });
        for alt in alt_leaders {
            cards.push(Card {
                card: alt,
                price: 4,
                basic_land: false,
                run: Some(RunCard {
                    presentation: None,
                    identity: ColorSet::ALL,
                    commander: true,
                }),
            });
        }
        cards.push(Card {
            card: ring,
            price: 4,
            basic_land: false,
            run: Some(RunCard {
                presentation: None,
                identity: ColorSet::default(),
                commander: false,
            }),
        });
        cards.push(Card {
            card: spare,
            price: 4,
            basic_land: false,
            run: Some(RunCard {
                presentation: None,
                identity: ColorSet::of(baylee_core::color::Color::White),
                commander: false,
            }),
        });
        let catalog = Catalog { cards };
        let starter = Collection {
            formation: Some(Formation {
                commander: leader,
                sideboard: Vec::new(),
            }),
            owned: vec![
                Stack {
                    card: leader,
                    copies: 1,
                },
                Stack {
                    card: ring,
                    copies: 1,
                },
                Stack {
                    card: spare,
                    copies: 1,
                },
            ],
            deck: vec![
                Stack {
                    card: plains,
                    copies: 34,
                },
                Stack {
                    card: island,
                    copies: 1,
                },
                Stack {
                    card: swamp,
                    copies: 1,
                },
                Stack {
                    card: mountain,
                    copies: 1,
                },
                Stack {
                    card: forest,
                    copies: 1,
                },
                Stack {
                    card: ring,
                    copies: 1,
                },
            ],
        };
        let mut starters = vec![starter];
        for alt in alt_leaders {
            starters.push(Collection {
                formation: Some(Formation {
                    commander: alt,
                    sideboard: Vec::new(),
                }),
                owned: vec![Stack {
                    card: alt,
                    copies: 1,
                }],
                deck: vec![Stack {
                    card: plains,
                    copies: 39,
                }],
            });
        }
        let mut identity = Identity::native(Variant::Tower, "ui-test", "synthetic", 22);
        identity.rules = "tower-5".into();
        let cohort = api::Cohort {
            id: Uuid::now_v7(),
            contract: domain::v2::package::Cohort::for_generation(
                identity,
                &catalog,
                domain::v2::generation::RUN,
            )
            .unwrap(),
            catalog,
            starters,
            offered: vec![0, 1, 2, 3, 4],
        };
        let raw = domain::v2::run::Run::begin_for(
            &cohort.catalog,
            cohort.starters[0].clone(),
            &[9; 32],
            domain::v2::generation::RUN,
        )
        .unwrap();
        let run = api::Run::project(Uuid::now_v7(), cohort.id, raw);

        let (handle, _jobs) = crate::lobby::central::Handle::queued_for_test();
        let mut state = LobbyState::default();
        crate::lobby::central::restored_for_test(&mut state, Vec::new());
        state.central.as_mut().unwrap().handle = Some(handle.clone());
        state.echoes.session = Some(handle);

        let read_cohorts = state
            .echoes
            .model
            .read(Uuid::now_v7(), Read::Cohorts(None))
            .unwrap();
        state.echoes.model.receive(
            &read_cohorts,
            Some(api::Response::Cohorts {
                consent: 1,
                cohorts: vec![cohort],
            }),
            None,
        );
        let read_run = state
            .echoes
            .model
            .read(Uuid::now_v7(), Read::Run(run.id))
            .unwrap();
        state.echoes.model.receive(
            &read_run,
            Some(api::Response::Run { run: Box::new(run) }),
            None,
        );
        assert!(open(&mut state));
        (state, spare)
    }

    #[test]
    fn tower_editor_sells_unused_cards_blocks_unsaved_removals_and_reconciles_pool() {
        let (mut state, _spare) = seeded_tower_editor();
        let plains_slot = state.lobby.builder().slot_of("Plains").unwrap();
        let ring_slot = state.lobby.builder().slot_of("Sol Ring").unwrap();
        let spare_slot = state.lobby.builder().slot_of("Wrath of God").unwrap();

        // Built cards are hidden from results by default; spare card is shown and sellable.
        assert_eq!(state.lobby.builder().results(), &[spare_slot]);
        assert_eq!(sell_status(&state, plains_slot), TowerSellStatus::Hidden);
        assert_eq!(sell_status(&state, ring_slot), TowerSellStatus::Hidden);
        let TowerSellStatus::Sellable {
            gold: spare_gold,
            busy: false,
        } = sell_status(&state, spare_slot)
        else {
            panic!("expected spare card to be sellable");
        };
        assert!(spare_gold > 0);

        // Removing Sol Ring locally without saving marks it `SaveRefitFirst` rather than live sellable.
        assert!(state.lobby.builder_mut().remove(ring_slot, Zone::Main));
        assert!(matches!(
            sell_status(&state, ring_slot),
            TowerSellStatus::SaveRefitFirst { .. }
        ));
        assert!(state.lobby.builder_mut().add(plains_slot, Zone::Main));
        assert!(state.lobby.builder().dirty());

        // Confirming sale of the spare card dispatches Command::SellCard and locks `busy`.
        state.build.confirm_sell = Some(spare_slot);
        assert!(press(&mut state, BuildPress::ConfirmSellTowerCard));
        assert!(state.build.confirm_sell.is_none());
        assert!(pending(&state));
        assert!(matches!(
            sell_status(&state, spare_slot),
            TowerSellStatus::Sellable { busy: true, .. }
        ));
    }

    #[test]
    fn confirmed_server_card_sale_reconciles_pool_while_preserving_unsaved_deck_edits() {
        let (mut state, spare) = seeded_tower_editor();
        let plains_slot = state.lobby.builder().slot_of("Plains").unwrap();
        let ring_slot = state.lobby.builder().slot_of("Sol Ring").unwrap();
        let spare_slot = state.lobby.builder().slot_of("Wrath of God").unwrap();

        let TowerSellStatus::Sellable {
            gold: spare_gold,
            busy: false,
        } = sell_status(&state, spare_slot)
        else {
            panic!("expected spare card to be sellable");
        };

        // Make a local unsaved edit on other cards before the sale settles.
        assert!(state.lobby.builder_mut().remove(ring_slot, Zone::Main));
        assert!(state.lobby.builder_mut().add(plains_slot, Zone::Main));
        assert!(state.lobby.builder().dirty());

        let work = state
            .echoes
            .command(Command::SellCard { card: spare })
            .expect("sell command work");
        state.echoes.editor.as_mut().unwrap().sent = Some(Sent::Sell {
            card: spare,
            card_name: "Wrath of God".into(),
            gold: spare_gold,
            prev_copies: 1,
        });

        let mut updated = state.echoes.model.run().unwrap().clone();
        updated.revision += 1;
        updated.economy.gold += spare_gold;
        updated.cards.owned.retain(|s| s.card != spare);
        state.echoes.model.receive(
            &work,
            Some(api::Response::Run {
                run: Box::new(updated),
            }),
            None,
        );

        reconcile(&mut state, 2.0);
        assert!(!pending(&state));
        assert!(
            state.lobby.builder().slot_of("Wrath of God").is_none(),
            "sold card removed from scoped builder pool"
        );
        assert!(
            state.lobby.builder().dirty() && state.lobby.builder().saveable(),
            "unsaved valid deck edits on other cards remain intact"
        );
    }
}
