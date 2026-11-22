//! History and restore are anchored to the version the player actually saw.
use super::{Account, Error, LibraryRequest, Versions, id};
use baylee_client_core::lobby::library::{CardCount, History, Reply, Revision, Snapshot};

pub(super) fn apply(
    account: &mut Account,
    versions: &mut Versions,
    request: LibraryRequest,
) -> Result<Reply, Error> {
    match request {
        LibraryRequest::History(key) => {
            let id = id(&key)?;
            let (head, history) = account.deck_history_page(id)?;
            let past = history
                .into_iter()
                .map(|save| {
                    Ok(Revision {
                        version: i32::try_from(save.revision).map_err(|_| Error::Rejected)?,
                        summary: save.summary,
                        superseded_at: save.superseded_at,
                        cards: save.cards,
                        sideboard: save.sideboard,
                        delta: None,
                        card_count: None,
                    })
                })
                .collect::<Result<_, Error>>()?;
            versions.history = Some((id, head.revision));
            Ok(Reply::History(
                key,
                History {
                    version: i32::try_from(head.revision).map_err(|_| Error::Rejected)?,
                    updated_at: head.updated_at,
                    past,
                    delta: None,
                    card_count: Some(CardCount {
                        main: head.copies,
                        side: head.side_copies,
                    }),
                },
            ))
        }
        LibraryRequest::Version(key, version) => {
            let deck = account.deck_version(
                id(&key)?,
                u64::try_from(version).map_err(|_| Error::Rejected)?,
            )?;
            Ok(Reply::Version(
                key,
                Snapshot {
                    version,
                    cards: deck.draft.cards,
                    sideboard: deck.draft.sideboard,
                    commanders: deck.draft.commanders,
                },
            ))
        }
        LibraryRequest::Restore(key, version) => {
            let id = id(&key)?;
            let (_, expected) = versions
                .history
                .filter(|(deck, _)| *deck == id)
                .ok_or(Error::Conflict)?;
            let saved = account.revert_deck(
                id,
                expected,
                u64::try_from(version).map_err(|_| Error::Rejected)?,
            )?;
            versions.history = Some((id, saved.revision));
            versions.edited.insert(id, saved.revision);
            versions.listed.insert(id, saved.revision);
            Ok(Reply::Restored(key))
        }
        LibraryRequest::House | LibraryRequest::Copy(_) => Err(Error::Rejected),
    }
}
