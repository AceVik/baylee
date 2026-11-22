//! Persist the server-issued create reservation in the encrypted draft before
//! submitting anything. A restarted editor can repeat the exact first write.
use super::{
    Account, DeckDraft, DeckSave, DeckSummary, Error, LobbyState, Mailbox, Reply, Target, Uuid,
    submit,
};
use crate::buildui::draft::private::ReservedCreate;

fn content(save: &DeckSave) -> DeckDraft {
    DeckDraft {
        name: save.draft.name.trim().to_owned(),
        format: save.format.clone(),
        cards: save.draft.main.clone(),
        sideboard: save.draft.side.clone(),
        commanders: save.draft.commanders.clone(),
        ..DeckDraft::default()
    }
}

pub(super) fn start(state: &mut LobbyState, mailbox: &Mailbox, save: &DeckSave) {
    crate::buildui::draft::Drafts::bind_private_owner(state);
    state.drafts.keep_visit(save.visit, &save.draft);
    if let Some(reservation) = state.drafts.create_for(save.visit) {
        match state.drafts.persist_create() {
            Ok(()) => send(state, mailbox, save, reservation),
            Err(error) => failed(state, mailbox, error),
        }
    } else {
        let save = Box::new(save.clone());
        submit(state, mailbox, Target::Autosave, move |account, _| {
            Ok(Reply::PrivateDeckReserved(save, account.reserve_deck_id()?))
        });
    }
}

pub(in crate::lobby) fn reserved(
    state: &mut LobbyState,
    mailbox: &Mailbox,
    save: &DeckSave,
    id: Uuid,
) {
    if !state.deck_saves.creating(save.visit, state.deck_epoch()) {
        return;
    }
    let reservation = ReservedCreate {
        id,
        first: content(save),
    };
    match state.drafts.reserve(save.visit, reservation.clone()) {
        Ok(()) => send(state, mailbox, save, reservation),
        Err(error) => failed(state, mailbox, error),
    }
}

fn failed(state: &mut LobbyState, mailbox: &Mailbox, error: Error) {
    let why = super::failure(state.lobby.lang(), error);
    crate::lobby::decksave::save_failed(state, mailbox, Some(why));
}

fn send(state: &LobbyState, mailbox: &Mailbox, save: &DeckSave, reservation: ReservedCreate) {
    let desired = content(save);
    submit(
        state,
        mailbox,
        Target::Autosave,
        move |account, versions| {
            let saved = commit(account, &reservation, desired)?;
            versions.edited.insert(saved.id, saved.revision);
            versions.listed.insert(saved.id, saved.revision);
            Ok(Reply::AutoSaved {
                deck_id: Some(saved.id.to_string()),
                archived: false,
            })
        },
    );
}

fn commit(
    account: &mut Account,
    reservation: &ReservedCreate,
    desired: DeckDraft,
) -> Result<DeckSummary, Error> {
    let id = Some(reservation.id);
    let first = account.save_deck_visit(id, None, reservation.first.clone(), false);
    if desired == reservation.first {
        return first;
    }
    match first {
        Ok(saved) => account.save_deck_visit(id, Some(saved.revision), desired, false),
        Err(Error::Conflict) => {
            // The expected first and second writes may both have landed.
            // A later independent edit always requires the reader to reload.
            let head = account.load_deck(reservation.id)?;
            if head.revision == 2 && head.draft == desired {
                account.save_deck_visit(id, Some(1), desired, false)
            } else {
                Err(Error::Conflict)
            }
        }
        Err(error) => Err(error),
    }
}
