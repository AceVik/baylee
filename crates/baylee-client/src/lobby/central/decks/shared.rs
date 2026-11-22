//! A published precon can be copied into the encrypted personal library.
//! Only its selected public identifier is sent back to its catalogue.
use super::{DeckDraft, Error, LibraryRequest, LobbyEvent, LobbyRequest, LobbyState};
use baylee_account::{GatewayHttp, decks::DeckSource};
use baylee_client_core::lobby::library;
use uuid::Uuid;
use zeroize::Zeroizing;

pub(super) enum Source {
    BuiltIn(Box<DeckDraft>),
    Published {
        origin: String,
        token: Zeroizing<String>,
        id: Uuid,
        version: u64,
    },
}

pub(super) fn published(state: &mut LobbyState, key: &str) -> Result<Source, Error> {
    let item = state
        .lobby
        .library()
        .house
        .iter()
        .find(|d| d.id == key)
        .ok_or(Error::Conflict)?
        .clone();
    let version = u64::try_from(item.version).map_err(|_| Error::Rejected)?;
    if state.central_without_gateway() {
        let lang = state.lobby.lang();
        let store = state.local_decks.as_mut().ok_or(Error::Conflict)?;
        let LobbyEvent::Library(library::Reply::Version(found, snapshot)) = store.perform(
            LobbyRequest::Library(LibraryRequest::Version(item.id.clone(), item.version)),
            lang,
        ) else {
            return Err(Error::Conflict);
        };
        if found != item.id || snapshot.version != item.version {
            return Err(Error::Conflict);
        }
        Ok(Source::BuiltIn(Box::new(DeckDraft {
            name: item.name,
            format: item.format,
            description: item.description,
            cards: snapshot.cards,
            sideboard: snapshot.sideboard,
            commanders: snapshot.commanders,
            copied_from: Some(DeckSource {
                gateway: None,
                account: None,
                deck: item.id,
                revision: version,
            }),
            ..DeckDraft::default()
        })))
    } else {
        Ok(Source::Published {
            origin: state.gateway.clone(),
            token: Zeroizing::new(state.lobby.token().ok_or(Error::Rejected)?.into()),
            id: super::id(key)?,
            version,
        })
    }
}

impl Source {
    pub(super) fn read(self) -> Result<DeckDraft, Error> {
        match self {
            Self::BuiltIn(draft) => Ok(*draft),
            Self::Published {
                origin,
                token,
                id,
                version,
            } => {
                let gateway = if crate::lobby::central::endpoint()?.legacy_origin().is_some() {
                    GatewayHttp::live(&origin)?
                } else {
                    GatewayHttp::development(&origin)?
                };
                gateway.published_deck(&token, id, version)
            }
        }
    }
}
