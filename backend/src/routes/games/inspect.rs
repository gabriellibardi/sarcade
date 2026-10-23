use std::path::Path;

use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use rusqlite::OptionalExtension;
use serde::Serialize;

use backend::game_runtime::{
    MAX_PLAYERS_PER_GAME, PlayerLimits, inspect_sgleam_players,
    version::{SGLEAM_BUILD_ID, SGLEAM_GIT_COMMIT, SINTERACT_PROTOCOL_VERSION},
};

use crate::{auth::current_user, state::AppState};

const MAX_SOURCE_CODE_LEN: usize = 200_000;

#[derive(Serialize)]
pub struct SourceInspection {
    pub min_players: u32,
    pub max_players: u32,
}

pub(crate) struct InspectedSource {
    pub runtime: String,
    pub min_players: i64,
    pub max_players: i64,
}

pub async fn inspect_source(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Result<Json<SourceInspection>, (StatusCode, String)> {
    current_user(&state.db, &jar)?.ok_or((
        StatusCode::UNAUTHORIZED,
        "Faça login para analisar um jogo".to_string(),
    ))?;

    let mut runtime_build_id = None;
    let mut source_code = None;
    let mut uploaded_filename = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?
    {
        match field.name().unwrap_or("") {
            "runtime_build_id" => {
                runtime_build_id = Some(field.text().await.unwrap_or_default());
            }
            "source_code" => {
                uploaded_filename = field.file_name().map(str::to_string);
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
                if bytes.len() > MAX_SOURCE_CODE_LEN {
                    return Err((StatusCode::BAD_REQUEST, "Código-fonte muito grande".into()));
                }
                source_code = Some(String::from_utf8(bytes.to_vec()).map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        "O código-fonte deve estar em UTF-8 válido".to_string(),
                    )
                })?);
            }
            _ => {}
        }
    }

    let runtime_build_id = runtime_build_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Selecione uma linguagem e uma versão".to_string(),
        ))?;
    let source_code = source_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Código-fonte é obrigatório".to_string(),
        ))?;

    let inspected = inspect_source_code(
        &state,
        &runtime_build_id,
        &source_code,
        uploaded_filename.as_deref(),
    )
    .await?;

    Ok(Json(SourceInspection {
        min_players: inspected.min_players as u32,
        max_players: inspected.max_players as u32,
    }))
}

pub(crate) async fn inspect_source_code(
    state: &AppState,
    runtime_build_id: &str,
    source_code: &str,
    uploaded_filename: Option<&str>,
) -> Result<InspectedSource, (StatusCode, String)> {
    let (runtime, source_extension) = {
        let conn = state.db.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Falha ao acessar o banco de dados".to_string(),
            )
        })?;
        conn.query_row(
            "
            SELECT runtime, source_extension
            FROM runtime_builds
            WHERE id = ?1
              AND status IN ('development', 'active')
              AND git_commit = ?2
              AND protocol_version = ?3
            ",
            rusqlite::params![
                runtime_build_id,
                SGLEAM_GIT_COMMIT,
                SINTERACT_PROTOCOL_VERSION
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Linguagem ou versão inválida ou indisponível".to_string(),
        ))?
    };

    if runtime_build_id != SGLEAM_BUILD_ID {
        return Err((
            StatusCode::BAD_REQUEST,
            "Linguagem ou versão inválida ou indisponível".into(),
        ));
    }

    if !has_expected_extension(uploaded_filename, &source_extension) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("O arquivo selecionado deve usar a extensão {source_extension}"),
        ));
    }
    if runtime != "sgleam" {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Esta linguagem ainda não oferece detecção automática de jogadores".into(),
        ));
    }

    let executable = state.sgleam_executable.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Ambiente de execução indisponível para analisar o jogo".to_string(),
    ))?;
    let _runtime_permit = state.reserve_runtime_process().map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "O servidor atingiu o limite de análises e partidas simultâneas; tente novamente em instantes"
                .to_string(),
        )
    })?;
    let limits = inspect_sgleam_players(
        executable.path().to_path_buf(),
        source_code.to_string(),
        source_extension,
    )
    .await
    .map_err(|error| {
        tracing::warn!(%error, "inspeção de código Sgleam falhou");
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("Não foi possível analisar o jogo: {error}"),
        )
    })?;

    validate_player_limits(limits)?;

    Ok(InspectedSource {
        runtime,
        min_players: i64::from(limits.min),
        max_players: i64::from(limits.max),
    })
}

fn validate_player_limits(limits: PlayerLimits) -> Result<(), (StatusCode, String)> {
    if limits.max <= MAX_PLAYERS_PER_GAME {
        return Ok(());
    }

    Err((
        StatusCode::UNPROCESSABLE_ENTITY,
        format!(
            "O jogo aceita até {} jogadores; o limite da plataforma é {MAX_PLAYERS_PER_GAME}",
            limits.max
        ),
    ))
}

fn has_expected_extension(filename: Option<&str>, expected_extension: &str) -> bool {
    let Some(filename) = filename else {
        return false;
    };
    let Some(actual_extension) = Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
    else {
        return false;
    };
    actual_extension.eq_ignore_ascii_case(expected_extension.trim_start_matches('.'))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use backend::game_runtime::{MAX_PLAYERS_PER_GAME, PlayerLimits};

    use super::{has_expected_extension, validate_player_limits};

    #[test]
    fn accepts_expected_source_extension() {
        assert!(has_expected_extension(Some("pong.gleam"), ".gleam"));
        assert!(has_expected_extension(Some("snake.PY"), ".py"));
    }

    #[test]
    fn rejects_missing_or_wrong_source_extension() {
        assert!(!has_expected_extension(Some("pong.py"), ".gleam"));
        assert!(!has_expected_extension(None, ".gleam"));
    }

    #[test]
    fn rejects_a_game_above_the_platform_player_limit() {
        assert!(
            validate_player_limits(PlayerLimits {
                min: 1,
                max: MAX_PLAYERS_PER_GAME,
            })
            .is_ok()
        );
        assert!(matches!(
            validate_player_limits(PlayerLimits {
                min: 1,
                max: MAX_PLAYERS_PER_GAME + 1,
            }),
            Err((StatusCode::UNPROCESSABLE_ENTITY, _))
        ));
    }
}
