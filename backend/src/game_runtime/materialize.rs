use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use rand::{Rng, distributions::Alphanumeric};
use sha2::{Digest, Sha256};

use super::RuntimeSpec;

const DIRECTORY_ATTEMPTS: usize = 16;
const RANDOM_NAME_BYTES: usize = 24;
const MAX_EXTENSION_BYTES: usize = 16;

/// Private, per-room copy of the approved source consumed by a runtime.
///
/// The directory is deleted when this value is dropped. It does not implement
/// `Debug`, because its path identifies a running room on the host.
pub struct MaterializedSource {
    directory: PathBuf,
    source_path: PathBuf,
}

impl MaterializedSource {
    pub fn create(spec: &RuntimeSpec, parent: &Path) -> io::Result<Self> {
        let extension = validate_extension(spec.source_extension())?;
        let directory = create_private_directory(parent)?;
        let source_path = directory.join(format!("game.{extension}"));

        let result = write_source(&source_path, spec.source_code())
            .and_then(|()| verify_digest(&source_path, spec.source_sha256()));

        if let Err(error) = result {
            let _ = fs::remove_dir_all(&directory);
            return Err(error);
        }

        Ok(Self {
            directory,
            source_path,
        })
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }
}

impl Drop for MaterializedSource {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.directory)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(
                path = %self.directory.display(),
                %error,
                "falha ao remover diretório temporário do runtime"
            );
        }
    }
}

fn validate_extension(extension: &str) -> io::Result<&str> {
    let Some(extension) = extension.strip_prefix('.') else {
        return Err(invalid_extension());
    };

    if extension.is_empty()
        || extension.len() > MAX_EXTENSION_BYTES
        || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(invalid_extension());
    }

    Ok(extension)
}

fn invalid_extension() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "runtime source extension is invalid",
    )
}

fn create_private_directory(parent: &Path) -> io::Result<PathBuf> {
    for _ in 0..DIRECTORY_ATTEMPTS {
        let suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(RANDOM_NAME_BYTES)
            .map(char::from)
            .collect();
        let directory = parent.join(format!("sarcade-runtime-{suffix}"));

        match create_directory(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique runtime directory",
    ))
}

#[cfg(unix)]
fn create_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_directory(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

fn write_source(path: &Path, source: &str) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file: File = options.open(path)?;
    file.write_all(source.as_bytes())?;
    file.flush()
}

fn verify_digest(path: &Path, expected: &[u8; 32]) -> io::Result<()> {
    let bytes = fs::read(path)?;
    let actual: [u8; 32] = Sha256::digest(bytes).into();

    if &actual == expected {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "materialized source digest does not match the approved revision",
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::MaterializedSource;
    use crate::game_runtime::RuntimeSpec;

    #[test]
    fn writes_the_exact_source_in_a_private_directory_and_cleans_it() {
        let parent = test_parent();
        let spec = spec(".gleam");
        let materialized = MaterializedSource::create(&spec, &parent).unwrap();
        let directory = materialized.directory().to_path_buf();

        assert_eq!(
            materialized.source_path().file_name().unwrap(),
            "game.gleam"
        );
        assert_eq!(
            fs::read_to_string(materialized.source_path()).unwrap(),
            spec.source_code()
        );
        assert!(
            !fs::metadata(materialized.directory())
                .unwrap()
                .permissions()
                .readonly()
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(materialized.directory())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(materialized.source_path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }

        drop(materialized);
        assert!(!directory.exists());
        fs::remove_dir(parent).unwrap();
    }

    #[test]
    fn rejects_an_extension_that_could_choose_a_path() {
        let parent = test_parent();
        let error = MaterializedSource::create(&spec(".gleam/../../segredo"), &parent)
            .err()
            .unwrap();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 0);
        fs::remove_dir(parent).unwrap();
    }

    fn test_parent() -> std::path::PathBuf {
        let parent = std::env::temp_dir().join(format!(
            "sarcade-materialize-test-{}",
            rand::random::<u64>()
        ));
        fs::create_dir(&parent).unwrap();
        parent
    }

    fn spec(extension: &str) -> RuntimeSpec {
        RuntimeSpec::new(
            1,
            2,
            "sgleam-dev".into(),
            "sgleam".into(),
            "desenvolvimento".into(),
            None,
            None,
            extension.into(),
            "development".into(),
            "pub fn main() { Nil }".into(),
            1,
            4,
        )
    }
}
