use super::*;

#[tokio::test]
async fn corrupted_committed_recovery_bytes_are_never_revealed_or_replaced() {
    let game = Uuid::now_v7();
    let mut config = setup(game, Prefix::default(), 2);
    let table = Table::new(
        game.to_string(),
        &config.package,
        &config.catalog,
        1,
        config.byte_limit,
        config.names.clone(),
    )
    .unwrap();
    let digest = config.package.digest(&config.catalog).unwrap();
    let mut records: Vec<_> = table
        .batch()
        .unwrap()
        .chunks()
        .map(<[u8]>::to_vec)
        .collect();
    for chunk in &records {
        config.prefix = config
            .prefix
            .extended(digest, config.byte_limit, chunk)
            .unwrap();
    }
    records[0][0] ^= 1;
    let (mut socket, worker) = launch(game).await;
    let story = tokio::spawn(async move {
        assert!(matches!(
            next(&mut socket).await.msg,
            Some(Msg::EngineHello(_))
        ));
        control(
            &mut socket,
            Control::Start {
                setup: Box::new(config),
            },
        )
        .await;
        let Msg::TowerFrame(frame) = next(&mut socket).await.msg.unwrap() else {
            panic!("only recovery requests before verified reconstruction")
        };
        let (message, _) = Control::decode(frame).unwrap();
        assert!(matches!(message, Control::Read { index: 0 }));
        send(
            &mut socket,
            &Control::Recovery { index: 0 }
                .envelope(records.remove(0))
                .unwrap(),
        )
        .await;
        let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .unwrap();
        assert!(
            !matches!(frame, Some(Ok(frame)) if frame.is_binary()),
            "no new record, seal or view follows changed evidence"
        );
    });
    let result = story.await;
    assert!(
        tokio::time::timeout(Duration::from_secs(5), worker)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    result.unwrap();
}

#[tokio::test]
async fn wrong_game_or_ack_generation_releases_no_native_view() {
    for wrong_game in [true, false] {
        let game = Uuid::now_v7();
        let (mut socket, worker) = launch(game).await;
        let story = tokio::spawn(async move {
            assert!(matches!(
                next(&mut socket).await.msg,
                Some(Msg::EngineHello(_))
            ));
            control(
                &mut socket,
                Control::Start {
                    setup: Box::new(setup(
                        if wrong_game { Uuid::now_v7() } else { game },
                        Prefix::default(),
                        1,
                    )),
                },
            )
            .await;
            if !wrong_game {
                assert!(matches!(
                    next(&mut socket).await.msg,
                    Some(Msg::TowerFrame(_))
                ));
                control(
                    &mut socket,
                    Control::Acknowledged {
                        acknowledgement: Acknowledged {
                            fence: 2,
                            prefix: Prefix::default(),
                        },
                    },
                )
                .await;
            }
            let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
                .await
                .unwrap();
            assert!(
                !matches!(frame, Some(Ok(frame)) if frame.is_binary()),
                "no view escapes a refused binding"
            );
        });
        let result = story.await;
        assert!(
            tokio::time::timeout(Duration::from_secs(5), worker)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        result.unwrap();
    }
}
