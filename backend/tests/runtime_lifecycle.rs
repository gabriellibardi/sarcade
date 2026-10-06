use std::{path::PathBuf, time::Duration};

use backend::game_runtime::{
    RuntimeCommand, RuntimeHandle, RuntimeInputEvent, RuntimeOutput, RuntimePlayer, RuntimeSpec,
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

#[tokio::test]
async fn missing_executable_fails_before_ready() {
    let missing = std::env::temp_dir().join(format!(
        "sarcade-runtime-that-does-not-exist-{}",
        std::process::id()
    ));
    let (_runtime, mut outputs) =
        RuntimeHandle::reserve_with_executable(runtime_spec(), missing);

    let output = next_output(&mut outputs).await;
    assert!(matches!(output, RuntimeOutput::Failed(_)));
}

#[tokio::test]
async fn input_before_start_ends_the_prepared_session() {
    let (runtime, mut outputs) = prepared_runtime().await;

    runtime
        .send(RuntimeCommand::Input {
            player_id: 1,
            event: RuntimeInputEvent::KeyDown { key: "a".into() },
        })
        .unwrap();

    let error = next_failure(&mut outputs).await;
    assert!(error.contains("antes de start"));
}

#[tokio::test]
async fn a_second_start_ends_the_existing_session_instead_of_relaunching() {
    let (runtime, mut outputs) = prepared_runtime().await;
    runtime.send(start_command()).unwrap();
    assert!(matches!(
        next_output(&mut outputs).await,
        RuntimeOutput::Frame { player_id: 1, .. }
    ));

    runtime.send(start_command()).unwrap();

    let error = next_failure(&mut outputs).await;
    assert!(error.contains("segundo start"));
}

async fn prepared_runtime() -> (RuntimeHandle, tokio::sync::mpsc::Receiver<RuntimeOutput>) {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_backend"));
    let (runtime, mut outputs) =
        RuntimeHandle::reserve_with_executable(runtime_spec(), executable);
    assert!(matches!(next_output(&mut outputs).await, RuntimeOutput::Ready));
    (runtime, outputs)
}

async fn next_failure(
    outputs: &mut tokio::sync::mpsc::Receiver<RuntimeOutput>,
) -> String {
    loop {
        match next_output(outputs).await {
            RuntimeOutput::Failed(error) => return error,
            RuntimeOutput::Frame { .. } => {}
            RuntimeOutput::Ready => panic!("runtime became ready twice"),
        }
    }
}

async fn next_output(
    outputs: &mut tokio::sync::mpsc::Receiver<RuntimeOutput>,
) -> RuntimeOutput {
    tokio::time::timeout(Duration::from_secs(5), outputs.recv())
        .await
        .expect("runtime output timed out")
        .expect("runtime output ended unexpectedly")
}

fn start_command() -> RuntimeCommand {
    RuntimeCommand::Start {
        game_id: 7,
        players: vec![RuntimePlayer {
            id: 1,
            nickname: "Ana".into(),
        }],
    }
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
