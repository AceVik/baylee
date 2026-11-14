//! Real native HTTP contacts and durable ciphertext, using the browser crypto SDK.
#![forbid(unsafe_code)]
use baylee_account::{
    Account, Endpoint, Error, chat_api as chat,
    sdk::chat::Chat,
    social::{ContactAction, Relation, SocialRequest, SocialResponse},
};
use central_crypto::chat::DeviceSecrets;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use tokio::runtime::Runtime;

fn signed_in(
    runtime: &Runtime,
    db: &DatabaseConnection,
    endpoint: &Endpoint,
    name: &str,
    password: &str,
) -> (Account, u32, Chat) {
    let row = runtime
        .block_on(db.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT tag FROM central_identity.accounts WHERE username_key = $1",
            [name.into()],
        )))
        .unwrap()
        .unwrap();
    let tag = u32::try_from(row.try_get::<i32>("", "tag").unwrap()).unwrap();
    // The preceding migration scenarios intentionally share this account and
    // may spend its eight OPAQUE evaluations per minute. Respect the real limit.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(65);
    let account = loop {
        match Account::login(endpoint.clone(), name.into(), password.into(), vec![]) {
            Ok(account) => break account,
            Err(Error::Busy) if std::time::Instant::now() < until => {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Err(error) => panic!("synthetic chat login: {error:?}"),
        }
    };
    let keys = account.chat_identity().unwrap();
    let crypto = Chat::new(DeviceSecrets::from_bytes(keys.bytes()), tag).unwrap();
    assert!(
        matches!(request(&account, chat::ChatRequest::Register { device: crypto.registration().unwrap() }),
        chat::ChatResponse::Registered { id } if id == crypto.device())
    );
    (account, tag, crypto)
}
fn request(account: &Account, request: chat::ChatRequest) -> chat::ChatResponse {
    let SocialResponse::Chat { response } =
        account.social(SocialRequest::Chat { request }).unwrap()
    else {
        panic!("typed chat response")
    };
    response
}
fn change(account: &Account, tag: u32, change: ContactAction) {
    assert!(matches!(
        account
            .social(SocialRequest::Contact { tag, change })
            .unwrap(),
        SocialResponse::Done
    ));
}
fn directory(account: &Account, crypto: &mut Chat, tags: &[u32]) {
    let chat::ChatResponse::Keys { keys } =
        request(account, chat::ChatRequest::Keys { tags: tags.into() })
    else {
        panic!("directory")
    };
    assert_eq!(keys.len(), tags.len());
    for key in keys {
        crypto.observe(key).unwrap();
    }
}
pub(super) fn run(runtime: &Runtime, db: &DatabaseConnection, endpoint: &Endpoint) {
    let (alice, a, mut ac) = signed_in(runtime, db, endpoint, "synthetic", super::PASSWORD);
    let (bob, b, mut bc) = signed_in(
        runtime,
        db,
        endpoint,
        "fresh_player",
        super::fresh::password(),
    );
    connect(&alice, &bob, a, b);
    directory(&alice, &mut ac, &[a, b]);
    directory(&bob, &mut bc, &[a, b]);
    let conv = format!("dm:{:04x}:{:04x}", a.min(b), a.max(b));
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let sealed = ac
        .seal(
            &conv,
            "Native HTTP → browser crypto: Grüße 🦊".into(),
            false,
            &[a, b],
            now,
        )
        .unwrap();
    let chat::ChatResponse::Sent { id, at } = request(
        &alice,
        chat::ChatRequest::Send {
            conv: conv.clone(),
            sealed: sealed.clone(),
        },
    ) else {
        panic!("receipt")
    };
    assert!(
        matches!(request(&alice, chat::ChatRequest::Send { conv: conv.clone(), sealed }), chat::ChatResponse::Sent { id: same, at: first } if same == id && first == at)
    );
    assert!(matches!(
        bob.social(SocialRequest::Chat {
            request: chat::ChatRequest::Inbox {
                device: ac.device().into(),
                after: 0
            }
        }),
        Err(Error::Rejected)
    ));
    let inbox = || {
        request(
            &bob,
            chat::ChatRequest::Inbox {
                device: bc.device().into(),
                after: 0,
            },
        )
    };
    let chat::ChatResponse::Inbox { messages, more } = inbox() else {
        panic!("inbox")
    };
    assert!(!more);
    assert_eq!(messages.len(), 1);
    let opened = bc.open(&messages[0]).unwrap();
    assert_eq!(opened.id, id);
    assert_eq!(opened.text, "Native HTTP → browser crypto: Grüße 🦊");
    let chat::ChatResponse::Inbox {
        messages: repeated, ..
    } = inbox()
    else {
        panic!("replay")
    };
    assert_eq!(repeated[0].seq, messages[0].seq);
    change(&bob, a, ContactAction::Block);
    let blocked = ac
        .seal(&conv, "blocked".into(), false, &[a, b], now)
        .unwrap();
    assert!(matches!(
        alice.social(SocialRequest::Chat {
            request: chat::ChatRequest::Send {
                conv,
                sealed: blocked
            }
        }),
        Err(Error::Rejected)
    ));
    let chat::ChatResponse::Inbox {
        messages: retained, ..
    } = inbox()
    else {
        panic!("retained")
    };
    assert_eq!(retained.len(), 1);
    assert_eq!(bc.open(&retained[0]).unwrap().id, id);
    alice.logout().unwrap();
    bob.logout().unwrap();
}

fn connect(alice: &Account, bob: &Account, a: u32, b: u32) {
    change(alice, b, ContactAction::Request);
    let SocialResponse::Contacts { contacts } = bob.social(SocialRequest::Contacts).unwrap() else {
        panic!("contacts")
    };
    assert!(
        contacts
            .iter()
            .any(|c| c.person.tag == a && c.relation == Relation::Incoming)
    );
    change(bob, a, ContactAction::Accept);
}
