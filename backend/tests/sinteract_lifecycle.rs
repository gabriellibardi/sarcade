use std::{path::PathBuf, time::Duration};

use backend::game_runtime::{
    RuntimeSpec, SinteractCommandError, SinteractHandle, SinteractOutput, SinteractPlayer,
    SinteractStatus,
};
use sinteract::view::FrameReader;

#[tokio::test]
async fn sinteract_session_reuses_the_prepared_process_and_targets_each_seat() {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_backend"));
    let (runtime, mut statuses) = SinteractHandle::reserve_fixture(runtime_spec(), executable);
    let mut changes = runtime.subscribe();

    assert_eq!(next_status(&mut statuses).await, SinteractStatus::Ready);

    assert_eq!(
        runtime
            .start(vec![
                SinteractPlayer {
                    id: 41,
                    nickname: "Ana".into(),
                },
                SinteractPlayer {
                    id: 41,
                    nickname: "Duplicada".into(),
                },
            ])
            .await,
        Err(SinteractCommandError::DuplicatePlayer),
    );

    runtime
        .start(vec![
            SinteractPlayer {
                id: 41,
                nickname: "Ana".into(),
            },
            SinteractPlayer {
                id: 73,
                nickname: "Beto".into(),
            },
        ])
        .await
        .unwrap();

    let old_ana = runtime.connect(41).await.unwrap();
    let ana = runtime.connect(41).await.unwrap();
    let beto = runtime.connect(73).await.unwrap();

    assert!(matches!(
        runtime.next_for(old_ana).await.unwrap(),
        SinteractOutput::Gone
    ));

    let ana_frame = next_frame(&runtime, &mut changes, ana).await;
    let beto_frame = next_frame(&runtime, &mut changes, beto).await;
    let mut ana_reader = FrameReader::new();
    let mut beto_reader = FrameReader::new();
    let ana_scene = ana_reader.read(&ana_frame).unwrap().unwrap();
    let beto_scene = beto_reader.read(&beto_frame).unwrap().unwrap();

    assert_eq!((ana_scene.width(), ana_scene.height()), (320.0, 240.0));
    assert_eq!((beto_scene.width(), beto_scene.height()), (320.0, 240.0));

    runtime.leave(ana).await.unwrap();
    runtime.leave(beto).await.unwrap();
    runtime.stop().await.unwrap();
}

#[tokio::test]
async fn sinteract_session_fails_before_ready_when_the_engine_cannot_start() {
    let missing = std::env::temp_dir().join(format!(
        "sarcade-sinteract-engine-that-does-not-exist-{}",
        std::process::id(),
    ));
    let (_runtime, mut statuses) = SinteractHandle::reserve(runtime_spec(), missing);

    assert!(matches!(
        next_status(&mut statuses).await,
        SinteractStatus::Failed(_),
    ));
}

async fn next_frame(
    runtime: &SinteractHandle,
    changes: &mut tokio::sync::watch::Receiver<u64>,
    conn: backend::game_runtime::SinteractConn,
) -> std::sync::Arc<[u8]> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match runtime.next_for(conn).await.unwrap() {
                SinteractOutput::Send(payload) => return payload,
                SinteractOutput::Idle => changes.changed().await.unwrap(),
                SinteractOutput::Gone => panic!("Sinteract connection ended before its frame"),
            }
        }
    })
    .await
    .expect("Sinteract frame timed out")
}

async fn next_status(
    statuses: &mut tokio::sync::mpsc::Receiver<SinteractStatus>,
) -> SinteractStatus {
    tokio::time::timeout(Duration::from_secs(5), statuses.recv())
        .await
        .expect("Sinteract status timed out")
        .expect("Sinteract status channel ended")
}

fn runtime_spec() -> RuntimeSpec {
    RuntimeSpec::new(
        7,
        11,
        "sgleam-sinteract-test".into(),
        "sgleam".into(),
        "sinteract-v1".into(),
        None,
        None,
        ".gleam".into(),
        "development".into(),
        "pub fn main() { Nil }".into(),
        1,
        2,
    )
}
