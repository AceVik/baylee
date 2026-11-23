use super::{Item, League, Source};
use crate::{
    hud::UiFonts,
    lobby::advent::{Book, Screen, api, draw},
    shellkit::controls::Kit,
};
use bevy::{ecs::system::RunSystemOnce, prelude::*};

pub(in crate::lobby) fn collection() -> Book {
    let item = |n, name: &str, source, kind| Item {
        id: [n; 16],
        kind,
        title: api::Text {
            de: name.into(),
            en: name.into(),
        },
        earned_at: 100,
        source,
        asset: api::AssetMeta {
            mime: api::Mime::Webp,
            tier: api::Finish::Standard,
            size: 64,
            width: 2,
            height: 2,
            digest: [n; 32],
            mat_framing: None,
        },
    };
    let mut book = Book::default();
    book.accept(
        api::Request::Collection { after: None },
        Ok(api::Response::Collection {
            items: vec![
                item(
                    1,
                    "Aurora",
                    Source::Advent {
                        year: 2026,
                        door: 8,
                    },
                    api::Kind::Sleeve,
                ),
                item(
                    2,
                    "Sol",
                    Source::Ranked {
                        league: League::Gold,
                        division: Some(2),
                    },
                    api::Kind::Sleeve,
                ),
                item(
                    3,
                    "Luna",
                    Source::Ranked {
                        league: League::Diamond,
                        division: Some(3),
                    },
                    api::Kind::Playmat,
                ),
                item(4, "Nova", Source::Unknown, api::Kind::Sleeve),
            ],
            next: None,
        }),
    );
    book
}

pub(in crate::lobby) fn kit(fonts: &UiFonts, german: bool) -> Kit<'_> {
    Kit {
        fonts,
        german,
        m: crate::lobby::header::kit_metrics(1100., 740., default(), default()),
    }
}

pub(in crate::lobby) fn words(world: &mut World) -> Vec<String> {
    world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect()
}

#[test]
fn each_recognized_rank_uses_its_real_divisions_and_keeps_future_sources_unlabelled() {
    use super::source_label;
    use baylee_client_core::Lang;
    let gold = |division| Source::Ranked {
        league: League::Gold,
        division,
    };
    let master = |division| Source::Ranked {
        league: League::Master,
        division,
    };
    assert_eq!(
        source_label(master(None), Lang::En).as_deref(),
        Some("Rank reward · Master")
    );
    for source in [
        Source::Unknown,
        gold(None),
        gold(Some(0)),
        gold(Some(5)),
        master(Some(2)),
        Source::Advent {
            year: 2026,
            door: 0,
        },
        Source::Ranked {
            league: League::Unranked,
            division: None,
        },
    ] {
        assert_eq!(source_label(source, Lang::En), None);
    }
}

#[test]
fn a_settlement_during_pagination_reloads_the_account_after_the_old_last_page() {
    let mut book = collection();
    let owned = book.inventory.clone();
    let cursor = Some(owned[0].id);
    book.accept(
        api::Request::Collection { after: None },
        Ok(api::Response::Collection {
            items: vec![owned[0].clone()],
            next: cursor,
        }),
    );
    book.invalidate_inventory();
    book.invalidate_inventory();
    assert_eq!(book.work.len(), 1);
    assert_eq!(
        book.work.pop_front(),
        Some(api::Request::Collection { after: cursor })
    );
    book.accept(
        api::Request::Collection { after: cursor },
        Ok(api::Response::Collection {
            items: owned[1..].to_vec(),
            next: None,
        }),
    );
    assert_eq!(
        book.work.pop_front(),
        Some(api::Request::Collection { after: None })
    );
    assert!(book.work.is_empty());
    assert_eq!(book.inventory, owned);
}

#[test]
fn collection_rows_and_reward_details_show_the_individually_awarded_rank() {
    for german in [true, false] {
        let rank = if german {
            "Rangbelohnung · Diamant III"
        } else {
            "Rank reward · Diamond III"
        };
        let sleeve = if german {
            "Kartenhülle"
        } else {
            "Card sleeve"
        };
        let mat = if german { "Spielmatte" } else { "Playmat" };
        for screen in [
            Screen::Gallery,
            Screen::Item([3; 16]),
            Screen::Item([4; 16]),
        ] {
            let mut app = App::new();
            let mut book = collection();
            book.screen = screen;
            app.insert_resource(book);
            app.world_mut()
                .run_system_once(move |mut cmd: Commands, book: Res<Book>| {
                    let parent = cmd.spawn(Node::default()).id();
                    let fonts = UiFonts::default();
                    draw::gallery(&mut cmd, parent, kit(&fonts, german), &book);
                })
                .unwrap();
            let text = words(app.world_mut());
            match screen {
                Screen::Gallery => {
                    assert!(text.contains(&format!("{mat} · {rank}")), "{text:?}");
                    let door = if german { "Tür 8" } else { "Door 8" };
                    assert!(
                        text.contains(&format!("{sleeve} · 2026 · {door}")),
                        "{text:?}"
                    );
                    assert!(text.iter().any(|s| s == sleeve));
                }
                Screen::Item(id) if id == [3; 16] => {
                    assert!(text.iter().any(|s| s == rank));
                    assert!(text.iter().any(|s| s == mat));
                }
                _ => {
                    assert!(text.iter().any(|s| s == "Nova"));
                    assert!(
                        text.iter()
                            .all(|s| !s.contains("2026") && !s.contains("Gold"))
                    );
                }
            }
        }
    }
}
