use axum::{Json, extract::State, http::StatusCode};

use crate::{db::Db, models::game::RuntimeBuildSummary};

pub async fn list_runtime_builds(
    State(db): State<Db>,
) -> Result<Json<Vec<RuntimeBuildSummary>>, (StatusCode, String)> {
    let conn = db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao acessar o banco de dados".to_string(),
        )
    })?;

    let mut stmt = conn
        .prepare(
            "
            SELECT
                id,
                runtime,
                display_name,
                display_version,
                git_commit,
                protocol_version,
                source_extension,
                status,
                is_default
            FROM runtime_builds
            WHERE status IN ('development', 'active')
            ORDER BY display_name, display_version
            ",
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let builds = stmt
        .query_map([], |row| {
            Ok(RuntimeBuildSummary {
                id: row.get(0)?,
                runtime: row.get(1)?,
                display_name: row.get(2)?,
                display_version: row.get(3)?,
                git_commit: row.get(4)?,
                protocol_version: row.get(5)?,
                source_extension: row.get(6)?,
                status: row.get(7)?,
                is_default: row.get::<_, i64>(8)? != 0,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(builds))
}
