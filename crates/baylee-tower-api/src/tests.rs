use super::*;

#[test]
fn player_wire_refuses_injected_authority_and_unbounded_bodies() {
    for input in [
        r#"{"operation":"withdraw_echoes","account":"foreign"}"#,
        r#"{"operation":"runs","before":null,"seed":[1]}"#,
        r#"{"operation":"cohorts","before":null,"origin":"https://evil.invalid"}"#,
        r#"{"operation":"settle","outcome":"victory"}"#,
        r#"{"operation":"command","run":"00000000-0000-7000-8000-000000000000","request":"00000000-0000-7000-8000-000000000001","revision":0,"command":{"kind":"claim","cards":[1,2,3],"gold":999}}"#,
        r#"{"operation":"command","run":"00000000-0000-7000-8000-000000000000","request":"00000000-0000-7000-8000-000000000001","revision":0,"command":{"kind":"freeze","frozen":true,"seed":[7]}}"#,
        r#"{"operation":"command","run":"00000000-0000-7000-8000-000000000000","request":"00000000-0000-7000-8000-000000000001","revision":0,"command":{"kind":"refresh","gold":999}}"#,
    ] {
        assert!(
            decode(input.as_bytes()).is_err(),
            "extended request accepted"
        );
    }
    assert!(decode(&vec![b' '; MAX_REQUEST_BYTES + 1]).is_err());
    assert_eq!(
        decode(br#"{"operation":"withdraw_echoes"}"#).unwrap(),
        Request::WithdrawEchoes {}
    );
}
