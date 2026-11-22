//! A private library lends the table a frozen deck without creating a gateway
//! library row. No private anchor or library operation crosses this boundary.

use crate::{Deck, DeckBody, Deserialize, ErrorBody, Json, Shared, StatusCode, auth, err};

/// A deck supplied for this table in place of a gateway library reference.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SubmittedDeck {
    /// @description Public name at this table, 1–64 bytes.
    name: String,
    /// @description Format of the submitted deck. Derived from its commanders when absent.
    format: Option<String>,
    /// @description Main-deck rows in the same form as a saved deck.
    cards: Vec<String>,
    /// @description Optional sideboard rows.
    #[serde(default)]
    sideboard: Vec<String>,
    /// @description Up to two commanders named as in a saved deck.
    #[serde(default)]
    commanders: Vec<String>,
    /// @description A sleeve uploaded by the caller on this gateway.
    sleeve: Option<String>,
    /// @description A playmat uploaded by the caller on this gateway.
    playmat: Option<String>,
}

pub(super) async fn chosen(
    state: &Shared,
    owner: &str,
    reference: Option<&str>,
    submitted: Option<SubmittedDeck>,
) -> Result<Option<(String, Deck)>, (StatusCode, Json<ErrorBody>)> {
    let body = match (reference, submitted) {
        (Some(_), Some(_)) => return Err(err(StatusCode::BAD_REQUEST, "choose one deck source")),
        (Some(reference), None) => return super::own_deck(state, owner, reference).await.map(Some),
        (None, None) => return Ok(None),
        (None, Some(deck)) => DeckBody {
            name: deck.name,
            cards: deck.cards,
            sideboard: deck.sideboard,
            commanders: deck.commanders,
            commander: None,
            format: deck.format,
            description: None,
            summary: None,
            sleeve: deck.sleeve,
            playmat: deck.playmat,
            versioned: false,
        },
    };
    super::validate_deck(&body)?;
    super::check_pictures(state, owner, &body).await?;
    let commanders = &body.commanders;
    let leaders = commanders
        .iter()
        .map(|name| baylee_cards::decks::by_name(name).and_then(baylee_cards::decks::leader_of))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "unknown commander"))?;
    let valid = match leaders.as_slice() {
        [] => true,
        [only] => only.eligible_alone,
        [a, b] => a.index != b.index && baylee_cards::decks::may_lead_together(a, b),
        _ => false,
    };
    if !valid {
        return Err(err(StatusCode::BAD_REQUEST, "invalid commander selection"));
    }
    let deck = Deck {
        id: auth::new_id(),
        account_id: owner.to_owned(),
        kind: "submitted".into(),
        name: body.name,
        format: body
            .format
            .unwrap_or_else(|| baylee_cards::decks::format_of(commanders).into()),
        origin: None,
        description: None,
        version: 1,
        offered: false,
        updated_at: auth::now_secs(),
        cards: body.cards,
        sideboard: body.sideboard,
        commanders: body.commanders,
        sleeve: body.sleeve,
        playmat: body.playmat,
    };
    let loaded = super::loaded_deck(&deck)?;
    state
        .blocks
        .check_loaded(&loaded, time::OffsetDateTime::now_utc())?;
    let mut preview = baylee_cards::decks::preset_for_all(0, &[&loaded]);
    super::hydrate_rules(state, &mut preview).await?;
    Ok(Some((deck.name.clone(), deck)))
}
