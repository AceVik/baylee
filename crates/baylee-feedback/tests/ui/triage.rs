//! Triage (`docs/feedback.md` §"Triage") over the UI's routes: filters with
//! several values, sorting, facets under the filter, revisions, duplicates,
//! the trash and its sweep, notes, bulk changes, what an admin opened,
//! saved views, crash groups, neighbours and the CSV export.

use sea_orm::{ConnectionTrait as _, DbBackend, Statement};
use serde_json::{Value, json};

use super::{Answer, GATEWAY, NAME, PASSWORD, Service, US, report};

/// A second admin, signed in; their cookie.
async fn second_admin(service: &Service) -> String {
    baylee_feedback::admin::add(&service.db, "astrid", PASSWORD)
        .await
        .expect("a second admin");
    let answer = service.login_as("astrid", PASSWORD, &[]).await;
    assert_eq!(answer.status, 200);
    answer.session_cookie().expect("a session cookie")
}

/// A crash report, its message and place as a client sends them.
fn crash(reporter: &str, message: &str, location: &str) -> Value {
    let mut r = report("crash", reporter, "it crashed", false);
    r["client"]["crash"] = json!({ "message": message, "location": location });
    r
}

impl Service {
    async fn get(&self, cookie: &str, path: &str) -> Answer {
        self.call("GET", path, &[("cookie", cookie.to_owned())], None)
            .await
    }

    async fn send(&self, cookie: &str, method: &str, path: &str, body: Value) -> Answer {
        self.call(
            method,
            path,
            &self.signed_in(cookie),
            Some(body.to_string()),
        )
        .await
    }

    async fn patch(&self, cookie: &str, id: &str, body: Value) -> Answer {
        self.send(cookie, "PATCH", &format!("/ui/api/reports/{id}"), body)
            .await
    }

    /// The ids the list answers for `query`, in its order.
    async fn ids(&self, cookie: &str, query: &str) -> Vec<String> {
        let answer = self.get(cookie, &format!("/ui/api/reports?{query}")).await;
        assert_eq!(answer.status, 200, "{query}: {}", answer.json());
        answer.json()["reports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_owned())
            .collect()
    }

    /// `(actor, action, detail)` of each audit line of report `id`.
    async fn trail(&self, cookie: &str, id: &str) -> Vec<(String, String, Option<String>)> {
        self.get(cookie, &format!("/ui/api/reports/{id}/audit"))
            .await
            .json()
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["actor"].as_str().unwrap().to_owned(),
                    e["action"].as_str().unwrap().to_owned(),
                    e["detail"].as_str().map(str::to_owned),
                )
            })
            .collect()
    }
}

fn line(actor: &str, action: &str, detail: Option<&str>) -> (String, String, Option<String>) {
    (
        actor.to_owned(),
        action.to_owned(),
        detail.map(str::to_owned),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one catalogue of filters
async fn several_values_filter_at_once_sort_and_count_under_the_filter() {
    let service = Service::start("multi", None).await;
    let cookie = service.login().await;
    let a = service
        .hand_in(GATEWAY, report("bug", "aaaa", "the swamp untapped", false))
        .await;
    let b = service
        .hand_in(GATEWAY, report("crash", "aaaa", "fell over", false))
        .await;
    let c = service
        .hand_in(US, report("improvement", "bbbb", "a better swamp", false))
        .await;
    let d = service
        .hand_in(US, report("bug", "cccc", "the island is grey", false))
        .await;
    assert_eq!(
        service
            .patch(
                &cookie,
                &a,
                json!({ "status": "triaged", "priority": "high", "assignee": NAME,
                        "labels": ["ui", "lands"] })
            )
            .await
            .status,
        200
    );
    service
        .patch(
            &cookie,
            &b,
            json!({ "priority": "critical", "labels": ["ui"] }),
        )
        .await;
    service
        .patch(
            &cookie,
            &c,
            json!({ "status": "resolved", "priority": "low" }),
        )
        .await;

    for (query, expected) in [
        ("status=new,triaged", vec![&d, &b, &a]),
        ("status=new", vec![&d, &b]),
        ("kind=bug,crash", vec![&d, &b, &a]),
        ("priority=high,none", vec![&d, &a]),
        ("priority=none", vec![&d]),
        ("assignee=none", vec![&d, &c, &b]),
        (&format!("assignee={NAME}"), vec![&a]),
        ("label=ui", vec![&b, &a]),
        ("label=lands", vec![&a]),
        ("q=swamp", vec![&c, &a]),
        ("q=SWAMP+untapped", vec![&a]),
        ("sort=priority", vec![&b, &a, &c, &d]),
        ("sort=priority&dir=asc", vec![&d, &c, &a, &b]),
        ("sort=status&dir=asc&kind=bug", vec![&d, &a]),
        ("sort=received&dir=asc", vec![&a, &b, &c, &d]),
    ] {
        assert_eq!(
            service.ids(&cookie, query).await,
            expected
                .iter()
                .map(|s| (*s).clone())
                .collect::<Vec<String>>(),
            "{query}"
        );
    }
    // The last eight characters of an id, as the UI shows it, find that
    // report.
    assert_eq!(
        service
            .ids(&cookie, &format!("q={}", &c[c.len() - 8..]))
            .await,
        std::slice::from_ref(&c)
    );
    for bad in [
        "status=new,lost",
        "kind=",
        "priority=urgent",
        "label=UI",
        "sort=size",
        "dir=sideways",
        "signature=nothex",
        "duplicate_of=nope",
        "channel=pigeon",
    ] {
        let answer = service
            .get(&cookie, &format!("/ui/api/reports?{bad}"))
            .await;
        assert_eq!(answer.status, 400, "{bad}");
    }

    // A facet counts under the filter but its own dimension: with bugs
    // picked, every kind is still offered, and the statuses are the bugs'.
    let facets = service.get(&cookie, "/ui/api/facets?kind=bug").await.json();
    assert_eq!(facets["total"], 2, "{facets}");
    let count = |list: &Value, value: &str| {
        list.as_array()
            .unwrap()
            .iter()
            .find(|c| c["value"] == value)
            .map_or(0, |c| c["count"].as_i64().unwrap())
    };
    assert_eq!(
        facets["kinds"].as_array().unwrap().len(),
        3,
        "bug, crash, improvement: {facets}"
    );
    assert_eq!(count(&facets["statuses"], "new"), 1, "{facets}");
    assert_eq!(count(&facets["statuses"], "triaged"), 1, "{facets}");
    assert_eq!(count(&facets["statuses"], "resolved"), 0, "{facets}");
    assert_eq!(count(&facets["labels"], "lands"), 1, "{facets}");
    assert_eq!(count(&facets["assignees"], NAME), 1, "{facets}");
    let unassigned = facets["assignees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["value"].is_null())
        .map(|c| c["count"].clone());
    assert_eq!(unassigned, Some(json!(1)), "an unassigned count: {facets}");
    let admins = service.get(&cookie, "/ui/api/admins").await.json();
    assert_eq!(admins, json!([NAME]));
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one report's fields, one by one
async fn a_change_writes_only_what_differs_and_a_stale_one_is_refused() {
    let service = Service::start("revision", None).await;
    let cookie = service.login().await;
    let id = service
        .hand_in(GATEWAY, report("bug", "ab12", "text", false))
        .await;
    let fresh = service
        .get(&cookie, &format!("/ui/api/reports/{id}"))
        .await
        .json();
    assert_eq!(fresh["revision"], 1);
    assert_eq!(fresh["labels"], json!([]));
    assert_eq!(fresh["notes"], 0);
    assert_eq!(fresh["note_list"], json!([]));
    assert!(fresh["done_at"].is_null() && fresh["deleted_at"].is_null());

    let changed = service
        .patch(
            &cookie,
            &id,
            json!({ "status": "resolved", "expected_revision": 1 }),
        )
        .await;
    assert_eq!(changed.status, 200);
    let changed = changed.json();
    assert_eq!(changed["revision"], 2);
    assert!(changed["done_at"].is_string(), "done when resolved");
    let stale = service
        .patch(
            &cookie,
            &id,
            json!({ "status": "triaged", "expected_revision": 1 }),
        )
        .await;
    assert_eq!(stale.status, 409, "another admin changed it meanwhile");
    let reopened = service
        .patch(
            &cookie,
            &id,
            json!({ "status": "triaged", "expected_revision": 2 }),
        )
        .await
        .json();
    assert!(reopened["done_at"].is_null(), "reopened is not done");

    // The same value again is no change: no revision, no audit line.
    let same = service
        .patch(&cookie, &id, json!({ "status": "triaged" }))
        .await
        .json();
    assert_eq!(same["revision"], 3);

    let labelled = service
        .patch(&cookie, &id, json!({ "labels": ["b", "a", "b"] }))
        .await
        .json();
    assert_eq!(labelled["labels"], json!(["a", "b"]), "sorted, once each");
    for bad in [
        json!({ "labels": ["Bad"] }),
        json!({ "labels": (0..17).map(|i| format!("l{i}")).collect::<Vec<_>>() }),
        json!({ "assignee": "nobody" }),
        json!({ "priority": "urgent" }),
        json!({ "duplicate_of": id }),
        json!({ "duplicate_of": "0190e0e0-0000-7000-8000-000000000000" }),
        json!({ "expected_revision": 3 }),
    ] {
        assert_eq!(
            service.patch(&cookie, &id, bad.clone()).await.status,
            400,
            "{bad}"
        );
    }
    let assigned = service
        .patch(
            &cookie,
            &id,
            json!({ "assignee": NAME, "priority": "critical" }),
        )
        .await
        .json();
    assert_eq!(
        (&assigned["assignee"], &assigned["priority"]),
        (&json!(NAME), &json!("critical"))
    );
    let cleared = service
        .patch(&cookie, &id, json!({ "assignee": null, "priority": null }))
        .await
        .json();
    assert!(cleared["assignee"].is_null() && cleared["priority"].is_null());
    assert_eq!(
        service.trail(&cookie, &id).await,
        [
            line(NAME, "status", Some("resolved")),
            line(NAME, "status", Some("triaged")),
            line(NAME, "labels", Some("a,b")),
            line(NAME, "assign", Some(NAME)),
            line(NAME, "priority", Some("critical")),
            line(NAME, "assign", None),
            line(NAME, "priority", None),
        ]
    );
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_duplicate_names_an_original_and_its_own_duplicates_follow_it() {
    let service = Service::start("duplicate", None).await;
    let cookie = service.login().await;
    let mut ids = Vec::new();
    for text in ["first", "second", "third", "fourth"] {
        ids.push(
            service
                .hand_in(GATEWAY, report("bug", "ab12", text, false))
                .await,
        );
    }
    let [a, b, c, d] = <[String; 4]>::try_from(ids).unwrap();
    let marked = service
        .patch(&cookie, &b, json!({ "duplicate_of": a }))
        .await
        .json();
    assert_eq!(marked["status"], "duplicate");
    assert_eq!(marked["duplicate_of"], a.as_str());
    assert!(marked["done_at"].is_string());
    assert_eq!(
        service
            .patch(&cookie, &c, json!({ "duplicate_of": b }))
            .await
            .status,
        400,
        "a duplicate is no original"
    );
    service
        .patch(&cookie, &c, json!({ "duplicate_of": a }))
        .await;
    assert_eq!(
        service.ids(&cookie, &format!("duplicate_of={a}")).await,
        [c.clone(), b.clone()]
    );
    // `a` turns out to duplicate `d`: its duplicates move on to `d`.
    service
        .patch(&cookie, &a, json!({ "duplicate_of": d }))
        .await;
    assert_eq!(
        service.ids(&cookie, &format!("duplicate_of={d}")).await,
        [c.clone(), b.clone(), a.clone()]
    );
    assert_eq!(
        service.trail(&cookie, &b).await.last().cloned(),
        Some(line(NAME, "duplicate", Some(&d)))
    );
    // Unmarked, it is no duplicate and goes back to triaged.
    let unmarked = service
        .patch(&cookie, &c, json!({ "duplicate_of": null }))
        .await
        .json();
    assert_eq!(
        (&unmarked["status"], &unmarked["duplicate_of"]),
        (&json!("triaged"), &Value::Null)
    );
    assert!(unmarked["done_at"].is_null());
    // Reopened, it is nobody's duplicate any more.
    let reopened = service
        .patch(&cookie, &b, json!({ "status": "new" }))
        .await
        .json();
    assert!(reopened["duplicate_of"].is_null(), "{reopened}");
    assert!(reopened["done_at"].is_null());
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_trash_keeps_a_report_until_it_is_restored_purged_or_swept() {
    let service = Service::start("trash", None).await;
    let cookie = service.login().await;
    let kept = service
        .hand_in(GATEWAY, report("bug", "ab12", "kept", false))
        .await;
    let old = service
        .hand_in(GATEWAY, report("bug", "ab12", "old", false))
        .await;
    for id in [&kept, &old] {
        let answer = service
            .call(
                "DELETE",
                &format!("/ui/api/reports/{id}"),
                &service.signed_in(&cookie),
                None,
            )
            .await;
        assert_eq!(answer.status, 204);
    }
    assert!(service.ids(&cookie, "").await.is_empty());
    assert_eq!(service.ids(&cookie, "trash=true").await.len(), 2);
    assert_eq!(
        service
            .patch(&cookie, &kept, json!({ "status": "triaged" }))
            .await
            .status,
        404,
        "nothing in the trash is changed"
    );
    let restored = service
        .call(
            "POST",
            &format!("/ui/api/reports/{kept}/restore"),
            &service.signed_in(&cookie),
            None,
        )
        .await;
    assert_eq!(restored.status, 204);
    assert_eq!(service.ids(&cookie, "").await, std::slice::from_ref(&kept));

    // Past its days in the trash, the next read of the list sweeps it.
    service
        .db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE feedback_report SET deleted_at = now() - make_interval(days => $2) \
             WHERE id = $1::uuid",
            [
                old.as_str().into(),
                (baylee_feedback::triage::TRASH_DAYS + 1).into(),
            ],
        ))
        .await
        .unwrap();
    assert!(service.ids(&cookie, "trash=true").await.is_empty());
    assert_eq!(
        service
            .get(&cookie, &format!("/ui/api/reports/{old}"))
            .await
            .status,
        404
    );
    assert_eq!(
        service.trail(&cookie, &old).await,
        [line(NAME, "trash", None), line("sweep", "delete", None)]
    );
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_note_is_kept_with_its_report_and_only_its_author_removes_it() {
    let service = Service::start("notes", None).await;
    let cookie = service.login().await;
    let other = second_admin(&service).await;
    let id = service
        .hand_in(GATEWAY, report("bug", "ab12", "text", false))
        .await;
    let path = format!("/ui/api/reports/{id}/notes");
    let notes = service
        .send(
            &cookie,
            "POST",
            &path,
            json!({ "text": "  seen it on 0.9 too  " }),
        )
        .await;
    assert_eq!(notes.status, 200);
    let notes = notes.json();
    assert_eq!(notes[0]["author"], NAME);
    assert_eq!(notes[0]["text"], "seen it on 0.9 too", "trimmed");
    for bad in [
        json!({ "text": "   " }),
        json!({ "text": "x".repeat(4001) }),
    ] {
        assert_eq!(service.send(&cookie, "POST", &path, bad).await.status, 400);
    }
    let note = notes[0]["id"].as_str().unwrap().to_owned();
    let summary = service.ids(&cookie, "").await;
    assert_eq!(summary, std::slice::from_ref(&id));
    let listed = service.get(&cookie, "/ui/api/reports").await.json();
    assert_eq!(listed["reports"][0]["notes"], 1);
    let detail = service
        .get(&cookie, &format!("/ui/api/reports/{id}"))
        .await
        .json();
    assert_eq!(detail["note_list"][0]["id"], note.as_str());
    assert_eq!(
        service
            .call(
                "DELETE",
                &format!("{path}/{note}"),
                &service.signed_in(&other),
                None
            )
            .await
            .status,
        404,
        "another admin's note"
    );
    let left = service
        .call(
            "DELETE",
            &format!("{path}/{note}"),
            &service.signed_in(&cookie),
            None,
        )
        .await;
    assert_eq!(left.status, 200);
    assert_eq!(left.json(), json!([]));
    // The token API shows no notes.
    let token = service
        .call(
            "GET",
            &format!("/reports/{id}"),
            &[("authorization", format!("Bearer {}", super::READ))],
            None,
        )
        .await
        .json();
    assert!(token.get("note_list").is_none(), "{token}");
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bulk_change_is_all_or_nothing() {
    let service = Service::start("bulk", None).await;
    let cookie = service.login().await;
    let mut ids = Vec::new();
    for text in ["one", "two", "three"] {
        ids.push(
            service
                .hand_in(GATEWAY, report("bug", "ab12", text, false))
                .await,
        );
    }
    let bulk = |body: Value| service.send(&cookie, "POST", "/ui/api/reports/bulk", body);
    let done =
        bulk(json!({ "ids": ids, "change": { "assignee": NAME, "labels_add": ["ui"] } })).await;
    assert_eq!(done.status, 200);
    assert_eq!(done.json()["changed"], 3);
    let again = bulk(json!({ "ids": ids, "change": { "labels_add": ["ui"] } })).await;
    assert_eq!(again.json()["changed"], 0, "nothing differed");
    let refused =
        bulk(json!({ "ids": ids, "change": { "assignee": "nobody", "priority": "high" } })).await;
    assert_eq!(refused.status, 400);
    assert!(
        service.ids(&cookie, "priority=high").await.is_empty(),
        "none of them changed"
    );
    for bad in [
        json!({ "ids": [], "action": "trash" }),
        json!({ "ids": ids, "action": "trash", "change": { "status": "new" } }),
        json!({ "ids": ids }),
        json!({ "ids": ids, "action": "burn" }),
        json!({ "ids": ids, "change": {} }),
        json!({ "ids": (0..201).map(|_| uuid::Uuid::now_v7()).collect::<Vec<_>>(), "action": "trash" }),
    ] {
        assert_eq!(bulk(bad.clone()).await.status, 400, "{bad}");
    }
    let moved =
        bulk(json!({ "ids": ids, "change": { "labels_remove": ["ui"], "labels_add": ["lands"] } }))
            .await
            .json();
    assert_eq!(moved["changed"], 3);
    assert_eq!(service.ids(&cookie, "label=lands").await.len(), 3);
    assert!(service.ids(&cookie, "label=ui").await.is_empty());

    let purge_early = bulk(json!({ "ids": ids, "action": "purge" })).await.json();
    assert_eq!(purge_early["changed"], 0, "only the trash is purged");
    let trashed = bulk(json!({ "ids": [ids[0], ids[1]], "action": "trash" }))
        .await
        .json();
    assert_eq!(trashed["changed"], 2);
    let purged = bulk(json!({ "ids": ids, "action": "purge" })).await.json();
    assert_eq!(purged["changed"], 2);
    assert_eq!(service.ids(&cookie, "").await, [ids[2].clone()]);
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn what_one_admin_opened_is_theirs() {
    let service = Service::start("seen", None).await;
    let cookie = service.login().await;
    let other = second_admin(&service).await;
    let a = service
        .hand_in(GATEWAY, report("bug", "ab12", "one", false))
        .await;
    let b = service
        .hand_in(GATEWAY, report("bug", "ab12", "two", false))
        .await;
    let seen = service
        .send(
            &cookie,
            "POST",
            "/ui/api/seen",
            json!({ "ids": [a], "seen": true }),
        )
        .await;
    assert_eq!(seen.status, 204);
    assert_eq!(
        service.ids(&cookie, "unseen=true").await,
        std::slice::from_ref(&b)
    );
    assert_eq!(service.ids(&other, "unseen=true").await.len(), 2);
    let listed = service.get(&cookie, "/ui/api/reports").await.json();
    let flag = |id: &str| {
        listed["reports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap()["seen"]
            .clone()
    };
    assert_eq!((flag(&a), flag(&b)), (json!(true), json!(false)));
    service
        .send(
            &cookie,
            "POST",
            "/ui/api/seen",
            json!({ "ids": [a], "seen": false }),
        )
        .await;
    assert_eq!(service.ids(&cookie, "unseen=true").await.len(), 2);

    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_view_is_its_admins_and_holds_a_filter_the_list_takes() {
    let service = Service::start("views", None).await;
    let cookie = service.login().await;
    let other = second_admin(&service).await;
    let views = service
        .send(
            &cookie,
            "POST",
            "/ui/api/views",
            json!({ "name": "Open bugs", "query": "?status=new,triaged&kind=bug" }),
        )
        .await;
    assert_eq!(views.status, 200);
    let views = views.json();
    assert_eq!(views[0]["query"], "status=new,triaged&kind=bug");
    let replaced = service
        .send(
            &cookie,
            "POST",
            "/ui/api/views",
            json!({ "name": "Open bugs", "query": "status=new" }),
        )
        .await
        .json();
    assert_eq!(replaced.as_array().unwrap().len(), 1, "one name, one view");
    assert_eq!(service.get(&other, "/ui/api/views").await.json(), json!([]));
    for bad in [
        json!({ "name": "", "query": "status=new" }),
        json!({ "name": "x", "query": "status=lost" }),
        json!({ "name": "x", "query": "sort=size" }),
    ] {
        assert_eq!(
            service
                .send(&cookie, "POST", "/ui/api/views", bad.clone())
                .await
                .status,
            400,
            "{bad}"
        );
    }
    let view = replaced[0]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        service
            .call(
                "DELETE",
                &format!("/ui/api/views/{view}"),
                &service.signed_in(&other),
                None
            )
            .await
            .status,
        404
    );
    let gone = service
        .call(
            "DELETE",
            &format!("/ui/api/views/{view}"),
            &service.signed_in(&cookie),
            None,
        )
        .await;
    assert_eq!(gone.json(), json!([]));
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crashes_group_by_signature_and_a_report_knows_its_neighbours() {
    let service = Service::start("crashes", None).await;
    let cookie = service.login().await;
    let first = service
        .hand_in(
            GATEWAY,
            crash(
                "aaaa",
                "index out of bounds: the len is 3 but the index is 5",
                "crates/x/src/table.rs:120:9",
            ),
        )
        .await;
    let second = service
        .hand_in(
            US,
            crash(
                "bbbb",
                "index out of bounds: the len is 7 but the index is 12",
                "crates/x/src/table.rs:131:13",
            ),
        )
        .await;
    let other = service
        .hand_in(
            GATEWAY,
            crash(
                "aaaa",
                "called `Option::unwrap()` on a `None` value",
                "src/a.rs:1:1",
            ),
        )
        .await;
    service
        .hand_in(GATEWAY, report("bug", "cccc", "not a crash", false))
        .await;
    let groups = service.get(&cookie, "/ui/api/crashes").await.json();
    let groups = groups.as_array().unwrap();
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert_eq!(groups[0]["count"], 2, "the same crash, numbers aside");
    assert_eq!(groups[0]["open"], 2);
    assert_eq!(groups[0]["latest"], second.as_str());
    assert_eq!(
        groups[0]["message"],
        "index out of bounds: the len is 7 but the index is 12"
    );
    let signature = groups[0]["signature"].as_str().unwrap().to_owned();
    assert_eq!(
        service
            .ids(&cookie, &format!("signature={signature}"))
            .await,
        [second.clone(), first.clone()]
    );
    // The words of a crash's message are searched as its text is.
    assert_eq!(
        service.ids(&cookie, "q=unwrap").await,
        std::slice::from_ref(&other)
    );

    let neighbours = service
        .get(
            &cookie,
            &format!("/ui/api/reports/{second}/neighbours?kind=crash"),
        )
        .await
        .json();
    assert_eq!(
        neighbours,
        json!({ "previous": other, "next": first, "position": 2, "total": 3 })
    );
    let outside = service
        .get(
            &cookie,
            &format!("/ui/api/reports/{second}/neighbours?kind=bug"),
        )
        .await
        .json();
    assert!(outside["position"].is_null(), "{outside}");
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_csv_export_follows_the_filter_and_holds_no_formula() {
    let service = Service::start("csv", None).await;
    let cookie = service.login().await;
    service
        .hand_in(
            GATEWAY,
            report("bug", "ab12", "=HYPERLINK(\"https://evil.example\")", false),
        )
        .await;
    service
        .hand_in(
            GATEWAY,
            report("feedback", "ab12", "nice, \"really\"", false),
        )
        .await;
    let answer = service.get(&cookie, "/ui/api/reports.csv?kind=bug").await;
    assert_eq!(answer.status, 200);
    assert_eq!(
        answer.header("content-type"),
        Some("text/csv; charset=utf-8")
    );
    assert!(
        answer
            .header("content-disposition")
            .unwrap()
            .starts_with("attachment")
    );
    let text = String::from_utf8(answer.body).unwrap();
    let rows: Vec<&str> = text.split("\r\n").filter(|l| !l.is_empty()).collect();
    assert_eq!(rows.len(), 2, "a header and the one bug: {text}");
    assert!(rows[0].starts_with("id,received,"), "{text}");
    assert!(
        rows[1].ends_with("\"'=HYPERLINK(\"\"https://evil.example\"\")\""),
        "{text}"
    );
    let all = service.get(&cookie, "/ui/api/reports.csv").await;
    assert!(
        String::from_utf8(all.body)
            .unwrap()
            .contains("\"nice, \"\"really\"\"\"")
    );
    service.close().await;
}

/// A report as a language-model seat (or anything else naming itself) sends
/// it: the client's `source`.
fn from(source: Value, text: &str) -> Value {
    let mut r = report("bug", "ab12", text, false);
    r["client"]["source"] = source;
    r
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_source_filter_shows_one_source_none_or_all_the_others() {
    let service = Service::start("source", None).await;
    let cookie = service.login().await;
    let seat = service
        .hand_in(GATEWAY, from(json!("llm-seat"), "the seat saw a fault"))
        .await;
    let player = service
        .hand_in(GATEWAY, report("bug", "cd34", "a player saw one", false))
        .await;
    let other = service
        .hand_in(GATEWAY, from(json!("a-bot"), "a bot saw one"))
        .await;
    // A source longer than 64 characters is read as its first 64, by the
    // filter, the facet and the summary alike.
    let long = "s".repeat(80);
    let cut = service
        .hand_in(GATEWAY, from(json!(long), "a long name"))
        .await;

    for (query, expected) in [
        ("source=llm-seat", vec![&seat]),
        ("source=!llm-seat", vec![&cut, &other, &player]),
        ("source=none", vec![&player]),
        ("source=!none", vec![&cut, &other, &seat]),
        (&format!("source={}", "s".repeat(64)), vec![&cut]),
        ("source=llm-seat&kind=crash", vec![]),
    ] {
        assert_eq!(
            service.ids(&cookie, query).await,
            expected
                .iter()
                .map(|s| (*s).clone())
                .collect::<Vec<String>>(),
            "{query}"
        );
    }
    for bad in [
        "source=",
        "source=!",
        "source=llm%01seat",
        &format!("source={}", "s".repeat(65)),
    ] {
        let answer = service
            .get(&cookie, &format!("/ui/api/reports?{bad}"))
            .await;
        assert_eq!(answer.status, 400, "{bad}");
    }

    // Every list row says its source; the facet counts each, `null` for
    // none, and keeps offering the others while one is picked.
    let list = service
        .get(&cookie, "/ui/api/reports?sort=received&dir=asc")
        .await
        .json();
    let sources: Vec<Value> = list["reports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["source"].clone())
        .collect();
    assert_eq!(
        sources,
        vec![
            json!("llm-seat"),
            Value::Null,
            json!("a-bot"),
            json!("s".repeat(64))
        ],
        "{list}"
    );
    let facets = service
        .get(&cookie, "/ui/api/facets?source=llm-seat")
        .await
        .json();
    assert_eq!(facets["total"], 1, "{facets}");
    let mut counted: Vec<(Value, i64)> = facets["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["value"].clone(), c["count"].as_i64().unwrap()))
        .collect();
    counted.sort_by_key(|(v, _)| v.to_string());
    assert_eq!(
        counted,
        vec![
            (json!("a-bot"), 1),
            (json!("llm-seat"), 1),
            (json!("s".repeat(64)), 1),
            (Value::Null, 1),
        ],
        "{facets}"
    );

    // The CSV names it too.
    let csv = service
        .get(&cookie, "/ui/api/reports.csv?source=llm-seat")
        .await;
    let text = String::from_utf8(csv.body).unwrap();
    let rows: Vec<&str> = text.split("\r\n").filter(|l| !l.is_empty()).collect();
    let header: Vec<&str> = rows[0].split(',').collect();
    let column = header
        .iter()
        .position(|c| *c == "source")
        .expect("a source column");
    let cells: Vec<&str> = rows[1].splitn(header.len(), ',').collect();
    assert_eq!(cells[column], "\"llm-seat\"", "{text}");
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_source_is_only_a_non_empty_string_and_the_filter_words_stay_the_filters() {
    let service = Service::start("source_shapes", None).await;
    let cookie = service.login().await;
    let mut none = Vec::new();
    for (shape, text) in [
        (json!(""), "an empty source"),
        (json!(42), "a number"),
        (json!({ "name": "a-bot" }), "an object"),
        (json!(true), "a flag"),
    ] {
        none.push(service.hand_in(GATEWAY, from(shape, text)).await);
    }
    let named_none = service
        .hand_in(GATEWAY, from(json!("none"), "a client calling itself none"))
        .await;
    let player = service
        .hand_in(GATEWAY, report("bug", "cd34", "a player saw one", false))
        .await;
    none.push(player);

    // Each of those reads as no source; a client that names itself `none`
    // is a source, and the filter's `none` still means "no source".
    let mut without = service
        .ids(&cookie, "source=none&sort=received&dir=asc")
        .await;
    without.sort();
    none.sort();
    assert_eq!(without, none);
    assert_eq!(
        service.ids(&cookie, "source=!none").await,
        vec![named_none.clone()]
    );
    let facets = service.get(&cookie, "/ui/api/facets").await.json();
    let mut counted: Vec<(Value, i64)> = facets["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["value"].clone(), c["count"].as_i64().unwrap()))
        .collect();
    counted.sort_by_key(|(v, _)| v.to_string());
    assert_eq!(
        counted,
        vec![(json!("none"), 1), (Value::Null, 5)],
        "{facets}"
    );
    service.close().await;
}
