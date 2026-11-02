//! Translate the machine's personal operations to the account-owned API.
use baylee_account::{Account, Error, chat_api as chat, groups, social as api};
use baylee_client_core::social::{HttpTag, Method};
use baylee_protocol::social as wire;
use serde_json::{Value, json};

pub(super) fn handles(tag: &HttpTag) -> bool {
    matches!(
        tag,
        HttpTag::RegisterDevice
            | HttpTag::OwnDevices
            | HttpTag::Keys(_)
            | HttpTag::Lookup(_)
            | HttpTag::AddFriend(_)
            | HttpTag::Friendship(_)
            | HttpTag::Group
    )
}

pub(super) fn tag(value: &str) -> Result<u32, Error> {
    let value = value.rsplit_once('#').map_or(value, |(_, tag)| tag);
    if !wire::valid_tag(value) {
        return Err(Error::Rejected);
    }
    u32::from_str_radix(value, 16)
        .ok()
        .filter(|value| (1..=0x7fff_ffff).contains(value))
        .ok_or(Error::Rejected)
}

fn field(body: Option<&Value>, key: &str) -> Result<String, Error> {
    body.and_then(|body| body.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(Error::Rejected)
}

pub(super) fn run(
    account: &Account,
    own: &str,
    purpose: &HttpTag,
    method: Method,
    path: &str,
    body: Option<&Value>,
) -> Result<(u16, Value), Error> {
    match purpose {
        HttpTag::RegisterDevice => {
            let device = serde_json::from_value(body.cloned().ok_or(Error::Rejected)?)
                .map_err(|_| Error::Rejected)?;
            match chat(account, chat::ChatRequest::Register { device })? {
                chat::ChatResponse::Registered { id } => Ok((201, json!({ "id": id }))),
                _ => Err(Error::Rejected),
            }
        }
        HttpTag::Keys(tags) => Ok((200, json!({ "keys": keys(account, tags)? }))),
        HttpTag::OwnDevices => {
            let keys = keys(account, &[own.to_owned()])?;
            let devices: Vec<_> = keys
                .into_iter()
                .flat_map(|account| account.devices)
                .map(|d| wire::DeviceRow {
                    id: d.id,
                    label: d.label,
                    added_at: d.added_at,
                    seen_at: d.added_at,
                })
                .collect();
            Ok((200, json!({ "devices": devices })))
        }
        HttpTag::Lookup(handle) => {
            let wanted = tag(handle)?;
            let api::SocialResponse::Person {
                person: Some(person),
            } = account.social(api::SocialRequest::Lookup { tag: wanted })?
            else {
                return Ok((404, json!({})));
            };
            let relations = contacts(account)?;
            let relation = relations
                .iter()
                .find(|row| row.person.tag == wanted)
                .map(|row| row.relation);
            let person = super::mapping::person(person);
            Ok((
                200,
                json!(wire::PlayerLookup {
                    tag: person.tag,
                    handle: person.handle,
                    friend: relation == Some(api::Relation::Friend),
                    incoming: relation == Some(api::Relation::Incoming),
                    outgoing: relation == Some(api::Relation::Outgoing),
                    blocked: relation == Some(api::Relation::Blocked)
                }),
            ))
        }
        HttpTag::AddFriend(handle) => {
            done(
                account,
                api::SocialRequest::Contact {
                    tag: tag(handle)?,
                    change: api::ContactAction::Request,
                },
            )?;
            Ok((201, json!({ "state": "pending" })))
        }
        HttpTag::Friendship(target) => {
            let change = if path == "/social/blocks" && method == Method::Post {
                api::ContactAction::Block
            } else if path.starts_with("/social/blocks/") && method == Method::Delete {
                api::ContactAction::Unblock
            } else if path.ends_with("/accept") && method == Method::Post {
                api::ContactAction::Accept
            } else if method == Method::Delete
                || (method == Method::Post && path.ends_with("/decline"))
            {
                api::ContactAction::Remove
            } else {
                return Err(Error::Rejected);
            };
            done(
                account,
                api::SocialRequest::Contact {
                    tag: tag(target)?,
                    change,
                },
            )?;
            Ok((204, json!({})))
        }
        HttpTag::Group => {
            let request = group(method, path, body)?;
            match account.social(api::SocialRequest::Group { request })? {
                api::SocialResponse::Group {
                    response: groups::GroupResponse::Created { id },
                } => Ok((201, json!({ "id": id }))),
                api::SocialResponse::Group {
                    response: groups::GroupResponse::Done,
                } => Ok((204, json!({}))),
                _ => Err(Error::Rejected),
            }
        }
        _ => Err(Error::Rejected),
    }
}

fn done(account: &Account, request: api::SocialRequest) -> Result<(), Error> {
    match account.social(request)? {
        api::SocialResponse::Done => Ok(()),
        _ => Err(Error::Rejected),
    }
}
pub(super) fn contacts(account: &Account) -> Result<Vec<api::Contact>, Error> {
    match account.social(api::SocialRequest::Contacts)? {
        api::SocialResponse::Contacts { contacts } => Ok(contacts),
        _ => Err(Error::Rejected),
    }
}
pub(super) fn chat(
    account: &Account,
    request: chat::ChatRequest,
) -> Result<chat::ChatResponse, Error> {
    match account.social(api::SocialRequest::Chat { request })? {
        api::SocialResponse::Chat { response } => Ok(response),
        _ => Err(Error::Rejected),
    }
}
pub(super) fn keys(account: &Account, tags: &[String]) -> Result<Vec<wire::AccountKeys>, Error> {
    let tags = tags
        .iter()
        .filter_map(|value| tag(value).ok())
        .collect::<Vec<_>>();
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    match chat(account, chat::ChatRequest::Keys { tags })? {
        chat::ChatResponse::Keys { keys } => keys
            .into_iter()
            .map(|key| {
                serde_json::to_value(key)
                    .and_then(serde_json::from_value)
                    .map_err(|_| Error::Rejected)
            })
            .collect(),
        _ => Err(Error::Rejected),
    }
}
pub(super) fn group(
    method: Method,
    path: &str,
    body: Option<&Value>,
) -> Result<groups::GroupRequest, Error> {
    let path = path.strip_prefix("/social/groups").ok_or(Error::Rejected)?;
    if path.is_empty() && method == Method::Post {
        return Ok(groups::GroupRequest::Create {
            name: field(body, "name")?,
        });
    }
    let parts: Vec<_> = path
        .strip_prefix('/')
        .ok_or(Error::Rejected)?
        .split('/')
        .collect();
    let id = *parts.first().ok_or(Error::Rejected)?;
    if !wire::valid_uuid(id) || id.as_bytes()[14] != b'7' {
        return Err(Error::Rejected);
    }
    let change = match (method, parts.as_slice()) {
        (Method::Patch, [_]) => groups::Change::Rename {
            name: field(body, "name")?,
        },
        (Method::Post, [_, "invites"]) => groups::Change::Invite {
            tag: tag(&field(body, "tag")?)?,
        },
        (Method::Post, [_, "join"]) => groups::Change::Join,
        (Method::Delete, [_, "invites", who]) => groups::Change::Withdraw { tag: tag(who)? },
        (Method::Delete, [_, "members", who]) => groups::Change::Remove { tag: tag(who)? },
        (Method::Post, [_, "owner"]) => groups::Change::Transfer {
            tag: tag(&field(body, "tag")?)?,
        },
        (Method::Delete, [_]) => groups::Change::Delete,
        _ => return Err(Error::Rejected),
    };
    Ok(groups::GroupRequest::Change {
        id: id.into(),
        change,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_registered_canonical_public_tags_cross_the_account_boundary() {
        assert_eq!(tag("Alice#0007"), Ok(7));
        assert_eq!(tag("7fffffff"), Ok(0x7fff_ffff));
        for value in [
            "0000",
            "ffffffff",
            "7",
            "000F",
            "alice",
            "Alice#0007/accept",
        ] {
            assert!(tag(value).is_err());
        }
    }
    #[test]
    fn group_routes_never_turn_a_malformed_or_unrelated_path_into_a_mutation() {
        let id = "01931234-5678-7000-8000-000000000001";
        assert!(matches!(
            group(Method::Post, &format!("/social/groups/{id}/join"), None),
            Ok(groups::GroupRequest::Change {
                change: groups::Change::Join,
                ..
            })
        ));
        assert!(matches!(
            group(
                Method::Delete,
                &format!("/social/groups/{id}/members/0007"),
                None
            ),
            Ok(groups::GroupRequest::Change {
                change: groups::Change::Remove { tag: 7 },
                ..
            })
        ));
        for path in [
            "/social/groups",
            "/social/groups/invalid",
            "/social/groups/../members/0007",
            "/social/blocks/0007",
        ] {
            assert!(group(Method::Delete, path, None).is_err());
        }
        assert!(
            group(
                Method::Post,
                &format!("/social/groups/{id}/owner"),
                Some(&json!({"tag":"ffffffff"}))
            )
            .is_err()
        );
    }
}
