//! Adapter between the shared editor and a revision-bound Tower refit. No deck
//! library writes, no independent filters, layout, drag code or card previews.
use super::{Command, LobbyState, Page, Phrase, api, domain};
use crate::lobby::{BuildPress, Hub};
use baylee_client_core::deckbuilder::{
    DeckBuilder,
    limited::{Commander, Holding},
};
use baylee_core::{deckrow::PrintChoice, ids::CardIndex};
use domain::cards::{Formation, Stack};
use std::collections::BTreeMap;
use uuid::Uuid;

pub(super) struct Session {
    run: Uuid,
    revision: u64,
    sent: Option<Command>,
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
        state.echoes.editor.as_mut().unwrap().sent = Some(command);
        state.build.save = crate::buildui::SaveState::Saving;
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
    // leaving an unacknowledged refit waits for its outcome.
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
        && (!state.echoes.model.uncertain()
            || state.build.save != crate::buildui::SaveState::Failed)
        && state
            .echoes
            .editor
            .as_ref()
            .is_some_and(|edit| edit.sent.is_some())
}

pub(super) fn reconcile(state: &mut LobbyState, now: f64) {
    if !state.lobby.scoped_builder() || state.echoes.model.busy() {
        return;
    }
    if state.echoes.model.uncertain() {
        state.build.save = crate::buildui::SaveState::Failed;
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
    let accepted = state.echoes.model.run().is_some_and(|run| {
        state.echoes.editor.as_ref().is_some_and(|edit| run.id == edit.run && run.revision > edit.revision)
            && matches!(&sent, Command::Refit { deck, formation } if run.cards.deck == *deck && run.cards.formation.as_ref() == Some(formation))
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
