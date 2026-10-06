use sha2::{Digest, Sha256};
use std::fmt::{self, Write};

/// Immutable description of the approved game revision selected for a room.
///
/// This type deliberately does not implement `Debug` or `Serialize`: it owns
/// submitted source code, which must not accidentally appear in logs or API
/// responses.
#[derive(Clone, PartialEq, Eq)]
pub struct RuntimeSpec {
    game_id: i64,
    revision_id: i64,
    runtime_build_id: String,
    runtime: String,
    display_version: String,
    git_commit: Option<String>,
    protocol_version: Option<i64>,
    source_extension: String,
    runtime_status: String,
    source_code: String,
    source_sha256: [u8; 32],
    min_players: i64,
    max_players: i64,
}

impl RuntimeSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        game_id: i64,
        revision_id: i64,
        runtime_build_id: String,
        runtime: String,
        display_version: String,
        git_commit: Option<String>,
        protocol_version: Option<i64>,
        source_extension: String,
        runtime_status: String,
        source_code: String,
        min_players: i64,
        max_players: i64,
    ) -> Self {
        let source_sha256 = Sha256::digest(source_code.as_bytes()).into();

        Self {
            game_id,
            revision_id,
            runtime_build_id,
            runtime,
            display_version,
            git_commit,
            protocol_version,
            source_extension,
            runtime_status,
            source_code,
            source_sha256,
            min_players,
            max_players,
        }
    }

    pub fn game_id(&self) -> i64 {
        self.game_id
    }

    pub fn revision_id(&self) -> i64 {
        self.revision_id
    }

    pub fn runtime_build_id(&self) -> &str {
        &self.runtime_build_id
    }

    pub fn runtime(&self) -> &str {
        &self.runtime
    }

    pub fn display_version(&self) -> &str {
        &self.display_version
    }

    pub fn git_commit(&self) -> Option<&str> {
        self.git_commit.as_deref()
    }

    pub fn protocol_version(&self) -> Option<i64> {
        self.protocol_version
    }

    pub fn source_extension(&self) -> &str {
        &self.source_extension
    }

    pub fn runtime_status(&self) -> &str {
        &self.runtime_status
    }

    pub fn source_code(&self) -> &str {
        &self.source_code
    }

    pub fn source_sha256(&self) -> &[u8; 32] {
        &self.source_sha256
    }

    pub fn source_sha256_hex(&self) -> String {
        let mut hex = String::with_capacity(self.source_sha256.len() * 2);
        for byte in self.source_sha256 {
            write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
        }
        hex
    }

    pub fn min_players(&self) -> i64 {
        self.min_players
    }

    pub fn max_players(&self) -> i64 {
        self.max_players
    }

    pub fn validate_protocol(
        &self,
        expected_protocol: u32,
    ) -> Result<(), RuntimeSpecError> {
        if self.runtime_status == "unavailable" {
            return Err(RuntimeSpecError::Unavailable);
        }

        if let Some(actual) = self.protocol_version
            && actual != i64::from(expected_protocol)
        {
            return Err(RuntimeSpecError::Protocol {
                expected: expected_protocol,
                actual,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeSpecError {
    Unavailable,
    Protocol { expected: u32, actual: i64 },
}

impl fmt::Display for RuntimeSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("runtime build is unavailable"),
            Self::Protocol { expected, actual } => write!(
                formatter,
                "runtime protocol {actual} is incompatible with expected protocol {expected}"
            ),
        }
    }
}

impl std::error::Error for RuntimeSpecError {}

#[cfg(test)]
mod tests {
    use super::RuntimeSpec;

    #[test]
    fn digest_identifies_the_exact_approved_source() {
        let first = spec("pub fn main() { 1 }");
        let same = spec("pub fn main() { 1 }");
        let different = spec("pub fn main() { 2 }");

        assert_eq!(first.source_sha256(), same.source_sha256());
        assert_ne!(first.source_sha256(), different.source_sha256());
        assert_eq!(first.source_sha256_hex().len(), 64);
    }

    #[test]
    fn rejects_an_unavailable_or_incompatible_runtime_before_start() {
        let mut unavailable = spec("pub fn main() { 1 }");
        unavailable.runtime_status = "unavailable".into();
        assert!(matches!(
            unavailable.validate_protocol(2),
            Err(super::RuntimeSpecError::Unavailable)
        ));

        let mut incompatible = spec("pub fn main() { 1 }");
        incompatible.protocol_version = Some(3);
        assert!(matches!(
            incompatible.validate_protocol(2),
            Err(super::RuntimeSpecError::Protocol {
                expected: 2,
                actual: 3
            })
        ));

        assert!(spec("pub fn main() { 1 }").validate_protocol(2).is_ok());
    }

    fn spec(source: &str) -> RuntimeSpec {
        RuntimeSpec::new(
            1,
            2,
            "sgleam-dev".into(),
            "sgleam".into(),
            "desenvolvimento".into(),
            None,
            None,
            ".gleam".into(),
            "development".into(),
            source.into(),
            1,
            4,
        )
    }
}
