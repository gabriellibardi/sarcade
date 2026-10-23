use std::{path::PathBuf, time::Duration};

#[cfg(feature = "embedded-sgleam")]
use backend::game_runtime::SgleamExecutable;
#[cfg(feature = "embedded-sgleam")]
use backend::game_runtime::inspect_sgleam_players;
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

#[cfg(feature = "embedded-sgleam")]
#[tokio::test]
async fn embedded_sgleam_runs_a_multiplayer_world() {
    let executable = SgleamExecutable::configured(None).unwrap().unwrap();
    let (runtime, mut statuses) =
        SinteractHandle::reserve(sgleam_runtime_spec(), executable.path().to_path_buf());
    let mut changes = runtime.subscribe();

    assert_eq!(next_status(&mut statuses).await, SinteractStatus::Ready);

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

    let ana = runtime.connect(41).await.unwrap();
    let beto = runtime.connect(73).await.unwrap();
    let mut ana_reader = FrameReader::new();
    let mut beto_reader = FrameReader::new();
    let ana_frame = next_frame(&runtime, &mut changes, ana).await;
    let beto_frame = next_frame(&runtime, &mut changes, beto).await;
    let ana_scene = ana_reader.read(&ana_frame).unwrap().unwrap();
    let beto_scene = beto_reader.read(&beto_frame).unwrap().unwrap();

    assert_eq!(ana_scene.height(), 240.0);
    assert_eq!(beto_scene.height(), 241.0);
    let ana_width = ana_scene.width();
    let beto_width = beto_scene.width();

    let next_ana_frame = next_frame(&runtime, &mut changes, ana).await;
    let next_beto_frame = next_frame(&runtime, &mut changes, beto).await;
    let next_ana_scene = ana_reader.read(&next_ana_frame).unwrap().unwrap();
    let next_beto_scene = beto_reader.read(&next_beto_frame).unwrap().unwrap();

    assert!(next_ana_scene.width() > ana_width);
    assert!(next_beto_scene.width() > beto_width);

    runtime.stop().await.unwrap();
}

#[cfg(feature = "embedded-sgleam")]
#[tokio::test]
async fn embedded_sgleam_inspection_reads_world_and_mworld_limits() {
    let executable = SgleamExecutable::configured(None).unwrap().unwrap();
    let single = inspect_sgleam_players(
        executable.path().to_path_buf(),
        r#"import sgleam/fill
import sgleam/image
import sgleam/world

pub fn main() {
  world.create(0, fn(_) { image.rectangle(10, 10, fill.black) })
  |> world.run()
}
"#
        .into(),
        ".gleam".into(),
    )
    .await
    .unwrap();
    let multiplayer = inspect_sgleam_players(
        executable.path().to_path_buf(),
        r#"import sgleam/fill
import sgleam/image
import sgleam/mworld

pub fn main() {
  mworld.create(0, min: 2, max: 4, to_image: fn(_, _) {
    image.rectangle(10, 10, fill.black)
  })
  |> mworld.run()
}
"#
        .into(),
        ".gleam".into(),
    )
    .await
    .unwrap();

    assert_eq!((single.min, single.max), (1, 1));
    assert_eq!((multiplayer.min, multiplayer.max), (2, 4));
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

#[cfg(feature = "embedded-sgleam")]
fn sgleam_runtime_spec() -> RuntimeSpec {
    RuntimeSpec::new(
        7,
        11,
        "sgleam-embedded-test".into(),
        "sgleam".into(),
        "sinteract-v1".into(),
        None,
        None,
        ".gleam".into(),
        "development".into(),
        r#"import sgleam/fill
import sgleam/image
import sgleam/mworld

pub fn main() {
  mworld.create(0, min: 2, max: 2, to_image: fn(state, player) {
    image.rectangle(
      320 + state,
      239 + mworld.player_index(player),
      fill.black,
    )
  })
  |> mworld.tick_rate(20)
  |> mworld.on_tick(fn(state) { state + 1 })
  |> mworld.run()
}
"#
        .into(),
        2,
        2,
    )
}
