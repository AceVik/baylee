//! Shape conversion only; cryptographic trust remains in the core machine.
use baylee_account::{Account, Error, chat_api as chat, groups, social as api};
use baylee_client_core::social::RetainedMessage;
use baylee_protocol::social as wire;

pub(super) fn person(person: api::Person) -> wire::Person {
    let tag = format!("{:04x}", person.tag);
    wire::Person {
        handle: format!("{}#{tag}", person.display_name),
        tag,
    }
}
pub(super) fn snapshot(account: &Account) -> Result<wire::Snapshot, Error> {
    let contacts = super::http::contacts(account)?;
    let api::SocialResponse::Group {
        response: groups::GroupResponse::Groups { snapshot },
    } = account.social(api::SocialRequest::Group {
        request: groups::GroupRequest::List,
    })?
    else {
        return Err(Error::Rejected);
    };
    let mut state = wire::Snapshot::default();
    for contact in contacts {
        let person = person(contact.person);
        match contact.relation {
            api::Relation::Friend => state.friends.push(wire::Friend {
                person,
                since: 0,
                presence: wire::Presence::offline(),
            }),
            api::Relation::Incoming => state.incoming.push(person),
            api::Relation::Outgoing => state.outgoing.push(person),
            api::Relation::Blocked => state.blocked.push(person),
        }
    }
    state.groups = snapshot
        .groups
        .into_iter()
        .map(|group| wire::GroupInfo {
            id: group.id,
            name: group.name,
            members: group
                .members
                .into_iter()
                .map(|member| wire::Member {
                    person: person(member.person),
                    owner: member.owner,
                })
                .collect(),
            invited: group.invited.into_iter().map(person).collect(),
        })
        .collect();
    state.group_invites = snapshot
        .invites
        .into_iter()
        .map(|invite| wire::GroupInvite {
            group: invite.group,
            name: invite.name,
            inviter: person(invite.inviter),
            at: invite.at,
        })
        .collect();
    Ok(state)
}
pub(super) fn delivery(delivery: chat::Delivery, device: &str) -> Result<RetainedMessage, Error> {
    let sealed = delivery.sealed;
    if sealed.wraps.len() != 1 || sealed.wraps[0].device != device {
        return Err(Error::Rejected);
    }
    if sealed.hdr.len() > wire::MAX_HEADER_BYTES * 2 {
        return Err(Error::Rejected);
    }
    let bytes = baylee_seal::b64::url::decode(&sealed.hdr).map_err(|_| Error::Rejected)?;
    let header: wire::Header = serde_json::from_slice(&bytes).map_err(|_| Error::Rejected)?;
    let sender = person(delivery.sender);
    if header.from != sender.tag
        || header.dev != delivery.device.id
        || delivery.seq == 0
        || delivery.seq > 9_007_199_254_740_991
        || header.ts.abs_diff(delivery.at) > 600_000
    {
        return Err(Error::Rejected);
    }
    Ok(RetainedMessage {
        seq: delivery.seq,
        at: delivery.at,
        conv: header.conv,
        from: wire::Sender {
            tag: sender.tag,
            handle: sender.handle,
            device: delivery.device.id,
        },
        sealed: wire::Delivered {
            hdr: sealed.hdr,
            n: sealed.n,
            ct: sealed.ct,
            epk: sealed.epk,
            sig: sealed.sig,
            k: sealed.wraps[0].k.clone(),
        },
    })
}
