use super::*;

#[test]
fn a_foretell_offer_is_not_silently_passed_as_nothing_to_do() {
    let (mut view, mut legal) = table(&[], &["Doomskar"]);
    view.seats[0].mana_pool.colorless = 2;
    assert!(matches!(
        judge(&view, legal.clone()),
        Verdict::Standing {
            why: Standing::NothingToDo,
            ..
        }
    ));
    legal.foretellable.push(o(20));
    assert!(matches!(judge(&view, legal), Verdict::Wake(_)));
}
