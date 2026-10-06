use std::{path::PathBuf, time::Duration};

use backend::game_runtime::{
    RuntimeCommand, RuntimeHandle, RuntimeOutput, RuntimePlayer, RuntimeSpec,
};

#[tokio::test]
async fn runtime_is_ready_before_start_and_reuses_the_same_process() {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_backend"));
    let (runtime, mut outputs) =
        RuntimeHandle::reserve_with_executable(runtime_spec(), executable);

    let ready = tokio::time::timeout(Duration::from_secs(5), outputs.recv())
        .await
        .expect("runtime preparation timed out")
        .expect("runtime output ended before ready");
    assert!(matches!(ready, RuntimeOutput::Ready));

    runtime
        .send(RuntimeCommand::Start {
            game_id: 7,
            players: vec![RuntimePlayer {
                id: 1,
                nickname: "Ana".into(),
            }],
        })
        .unwrap();

    let first_frame = tokio::time::timeout(Duration::from_secs(5), outputs.recv())
        .await
        .expect("first frame timed out")
        .expect("runtime ended before its first frame");
    assert!(matches!(
        first_frame,
        RuntimeOutput::Frame { player_id: 1, .. }
    ));

    runtime.send(RuntimeCommand::Stop).unwrap();

    let mut saw_final_frame = false;
    loop {
        let output = tokio::time::timeout(Duration::from_secs(5), outputs.recv())
            .await
            .expect("runtime shutdown timed out");
        match output {
            Some(RuntimeOutput::Frame { player_id: 1, .. }) => saw_final_frame = true,
            Some(RuntimeOutput::Frame { player_id, .. }) => {
                panic!("unexpected frame for player {player_id}")
            }
            Some(RuntimeOutput::Ready) => panic!("runtime became ready twice"),
            Some(RuntimeOutput::Failed(error)) => panic!("runtime failed: {error}"),
            None => break,
        }
    }

    assert!(saw_final_frame);
}

fn runtime_spec() -> RuntimeSpec {
    RuntimeSpec::new(
        7,
        11,
        "sgleam-dev".into(),
        "sgleam".into(),
        "desenvolvimento".into(),
        None,
        Some(2),
        ".gleam".into(),
        "development".into(),
        "pub fn main() { Nil }".into(),
        1,
        1,
    )
}
