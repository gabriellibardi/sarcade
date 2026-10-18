use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "embedded-sgleam")]
use std::{
    fs::{File, OpenOptions},
    io::Write,
};

#[cfg(feature = "embedded-sgleam")]
use rand::{Rng, distributions::Alphanumeric};

use super::version::{SGLEAM_GIT_COMMIT, SGLEAM_VERSION};

#[cfg(feature = "embedded-sgleam")]
const DIRECTORY_ATTEMPTS: usize = 16;
#[cfg(feature = "embedded-sgleam")]
const RANDOM_NAME_BYTES: usize = 24;

#[cfg(feature = "embedded-sgleam")]
const EMBEDDED_SGLEAM: &[u8] = include_bytes!(env!("SARCADE_SGLEAM_BINARY"));

#[derive(Clone)]
pub struct SgleamExecutable {
    inner: Arc<ExecutableInner>,
}

struct ExecutableInner {
    path: PathBuf,
    directory: Option<PathBuf>,
}

impl SgleamExecutable {
    pub fn configured(external: Option<PathBuf>) -> io::Result<Option<Self>> {
        match external {
            Some(path) => Ok(Some(Self::external(path))),
            None => Self::embedded(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    pub fn verify_build_identity(&self) -> io::Result<()> {
        let output = std::process::Command::new(self.path())
            .arg("--version")
            .output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "o executável Sgleam falhou ao informar sua versão",
            ));
        }

        let version = String::from_utf8(output.stdout).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "a versão do executável Sgleam não é UTF-8 válido",
            )
        })?;
        if !matches_expected_build(&version) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "o executável não corresponde ao Sgleam {SGLEAM_VERSION} ({}) fixado pelo Sarcade",
                    short_commit()
                ),
            ));
        }

        Ok(())
    }

    fn external(path: PathBuf) -> Self {
        Self {
            inner: Arc::new(ExecutableInner {
                path,
                directory: None,
            }),
        }
    }

    #[cfg(feature = "embedded-sgleam")]
    fn embedded() -> io::Result<Option<Self>> {
        let directory = create_private_directory(&std::env::temp_dir())?;
        let path = directory.join("sgleam");

        if let Err(error) = write_executable(&path, EMBEDDED_SGLEAM) {
            let _ = fs::remove_dir_all(&directory);
            return Err(error);
        }

        Ok(Some(Self {
            inner: Arc::new(ExecutableInner {
                path,
                directory: Some(directory),
            }),
        }))
    }

    #[cfg(not(feature = "embedded-sgleam"))]
    fn embedded() -> io::Result<Option<Self>> {
        Ok(None)
    }
}

fn matches_expected_build(version: &str) -> bool {
    version.starts_with(&format!("Version: {SGLEAM_VERSION} ("))
        && version
            .trim_end()
            .ends_with(&format!(", {})", short_commit()))
}

fn short_commit() -> &'static str {
    &SGLEAM_GIT_COMMIT[..7]
}

impl Drop for ExecutableInner {
    fn drop(&mut self) {
        let Some(directory) = &self.directory else {
            return;
        };

        if let Err(error) = fs::remove_dir_all(directory)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(
                path = %directory.display(),
                %error,
                "falha ao remover runtime Sgleam incorporado"
            );
        }
    }
}

#[cfg(feature = "embedded-sgleam")]
fn create_private_directory(parent: &Path) -> io::Result<PathBuf> {
    for _ in 0..DIRECTORY_ATTEMPTS {
        let suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(RANDOM_NAME_BYTES)
            .map(char::from)
            .collect();
        let directory = parent.join(format!("sarcade-sgleam-{suffix}"));

        match create_directory(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique Sgleam directory",
    ))
}

#[cfg(all(feature = "embedded-sgleam", unix))]
fn create_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(all(feature = "embedded-sgleam", not(unix)))]
fn create_directory(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

#[cfg(feature = "embedded-sgleam")]
fn write_executable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o700);
    }

    let mut file: File = options.open(path)?;
    file.write_all(bytes)?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::{SgleamExecutable, matches_expected_build, short_commit};
    use crate::game_runtime::version::SGLEAM_VERSION;

    #[test]
    fn an_external_executable_needs_no_copy() {
        let path = std::path::PathBuf::from("/runtime/sgleam");
        let executable = SgleamExecutable::configured(Some(path.clone()))
            .unwrap()
            .unwrap();

        assert_eq!(executable.path(), path);
    }

    #[test]
    fn accepts_only_the_version_and_commit_fixed_by_the_backend() {
        let expected = format!(
            "Version: {SGLEAM_VERSION} (using gleam 1, stdlib 1 and quickjs 1, 2026-10-18, {})\n",
            short_commit()
        );

        assert!(matches_expected_build(&expected));
        assert!(!matches_expected_build(
            "Version: 999.0.0 (using gleam 1, stdlib 1 and quickjs 1, 2026-10-18, deadbee)\n"
        ));
    }

    #[cfg(not(feature = "embedded-sgleam"))]
    #[test]
    fn a_development_build_has_no_implicit_executable() {
        assert!(SgleamExecutable::configured(None).unwrap().is_none());
    }

    #[cfg(feature = "embedded-sgleam")]
    #[test]
    fn embedded_sgleam_executable_matches_the_pinned_build_and_is_cleaned_up() {
        let executable = SgleamExecutable::configured(None).unwrap().unwrap();
        let path = executable.path().to_path_buf();
        let directory = path.parent().unwrap().to_path_buf();

        let output = std::process::Command::new(&path)
            .arg("--version")
            .output()
            .unwrap();

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).starts_with("Version: "));
        executable.verify_build_identity().unwrap();

        let clone = executable.clone();
        drop(executable);
        assert!(path.exists());
        drop(clone);
        assert!(!directory.exists());
    }
}
