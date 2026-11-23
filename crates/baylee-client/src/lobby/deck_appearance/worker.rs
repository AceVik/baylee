//! The account worker applies one cosmetic slot to the exact private deck.
use super::central::Handle;
use baylee_account::{
    Account, Error,
    advent::{AssetMeta, AssetScope, Equipment, Id, Kind, Request, Response},
};
use std::sync::mpsc::{self, Receiver};
use uuid::Uuid;

#[derive(Clone)]
pub(super) struct Deck {
    pub id: Uuid,
    pub name: String,
    pub look: Equipment,
}

pub(super) enum Job {
    List,
    Read(Uuid),
    Apply {
        deck: Uuid,
        kind: Kind,
        previous: Option<Id>,
        selected: Option<Box<(Id, AssetMeta)>>,
    },
}

pub(super) enum Reply {
    Decks(Vec<(Uuid, String)>),
    Loaded(Deck),
    Saved(Deck),
}

pub(super) fn slot(look: Equipment, kind: Kind) -> Option<Id> {
    match kind {
        Kind::Sleeve => look.sleeve,
        Kind::Playmat => look.playmat,
    }
}

fn replace(
    look: &mut Equipment,
    kind: Kind,
    previous: Option<Id>,
    chosen: Option<Id>,
) -> Result<bool, Error> {
    let current = match kind {
        Kind::Sleeve => &mut look.sleeve,
        Kind::Playmat => &mut look.playmat,
    };
    if *current == chosen {
        return Ok(false);
    }
    if *current != previous {
        return Err(Error::Conflict);
    }
    *current = chosen;
    Ok(true)
}

fn apply(
    account: &mut Account,
    owner: &Handle,
    deck: Uuid,
    kind: Kind,
    previous: Option<Id>,
    selected: Option<(Id, AssetMeta)>,
) -> Result<Reply, Error> {
    // The already verified digest keeps this a tiny authenticated ownership
    // read. Even a stale local inventory cannot equip a revoked item.
    if let Some((item, asset)) = selected {
        match account.advent(Request::Asset {
            scope: AssetScope::Owned { item },
            offset: 0,
            cached: Some(asset.digest),
        })? {
            Response::Cached { asset: confirmed } if confirmed == asset => {}
            _ => return Err(Error::Conflict),
        }
    }
    let current = account.load_deck(deck)?;
    let mut draft = current.draft;
    let changed = replace(&mut draft.look, kind, previous, selected.map(|s| s.0))?;
    let result = Deck {
        id: deck,
        name: draft.name.clone(),
        look: draft.look,
    };
    if changed {
        let saved = account.save_deck(Some(deck), Some(current.revision), draft)?;
        owner.appearance_saved(deck, current.revision, saved.revision);
    }
    Ok(Reply::Saved(result))
}

pub(super) fn submit(owner: &Handle, job: Job) -> Result<Receiver<Result<Reply, Error>>, Error> {
    let (send, recv) = mpsc::sync_channel(1);
    let handle = owner.clone();
    let accepted = owner.submit(move |account| {
        let result = match job {
            Job::List => account.list_decks().map(|rows| {
                let mut decks: Vec<_> = rows.into_iter().map(|d| (d.id, d.name)).collect();
                decks.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
                Reply::Decks(decks)
            }),
            Job::Read(id) => account.load_deck(id).map(|s| {
                Reply::Loaded(Deck {
                    id,
                    name: s.draft.name,
                    look: s.draft.look,
                })
            }),
            Job::Apply {
                deck,
                kind,
                previous,
                selected,
            } => apply(
                account,
                &handle,
                deck,
                kind,
                previous,
                selected.map(|picked| *picked),
            ),
        };
        let _ = send.send(result);
    });
    accepted.then_some(recv).ok_or(Error::Busy)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn advent_appearance_keeps_ranked_registration_and_elo_exactly_equal() {
        use baylee_account::decks::DeckDraft;
        use baylee_cards::ranked::{rating_pool, registration};
        let original = DeckDraft {
            name: "A".into(),
            format: "standard".into(),
            cards: vec!["60 Forest".into()],
            ..Default::default()
        };
        let mut worn = original.clone();
        worn.name = "B".into();
        worn.look = Equipment {
            sleeve: Some([5; 16]),
            playmat: Some([7; 16]),
        };
        let registered = |draft: &DeckDraft| {
            registration(
                &draft.format,
                &draft.cards,
                &draft.sideboard,
                &draft.commanders,
            )
            .unwrap()
        };
        let a = registered(&original);
        let b = registered(&worn);
        assert_eq!(a.digest, b.digest);
        assert_eq!(rating_pool(&a), rating_pool(&b));
    }
    #[test]
    fn edits_one_slot_without_overwriting_a_newer_appearance() {
        let a = Some([1; 16]);
        let b = Some([2; 16]);
        let mut look = Equipment {
            sleeve: a,
            playmat: b,
        };
        assert_eq!(
            replace(&mut look, Kind::Sleeve, None, b),
            Err(Error::Conflict)
        );
        assert_eq!(replace(&mut look, Kind::Sleeve, None, a), Ok(false));
        assert_eq!(replace(&mut look, Kind::Sleeve, a, None), Ok(true));
        assert_eq!(
            look,
            Equipment {
                sleeve: None,
                playmat: b
            }
        );
    }
}
