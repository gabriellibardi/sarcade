use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::RuntimeSpec;

#[derive(Clone, Default)]
pub struct RuntimeCatalog {
    runtimes: Arc<HashMap<String, InstalledRuntime>>,
}

#[derive(Clone)]
pub struct InstalledRuntime {
    id: String,
    runtime: String,
    display_name: String,
    display_version: String,
    git_commit: String,
    protocol_version: u32,
    source_extension: String,
    status: String,
    is_default: bool,
    executable: PathBuf,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    runtimes: Vec<ManifestRuntime>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestRuntime {
    id: String,
    runtime: String,
    display_name: String,
    display_version: String,
    git_commit: String,
    protocol_version: u32,
    source_extension: String,
    status: String,
    #[serde(default)]
    is_default: bool,
    executable: PathBuf,
    sha256: String,
}

impl RuntimeCatalog {
    pub fn load(path: &Path) -> io::Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("não foi possível ler {}: {error}", path.display()),
            )
        })?;
        let manifest: Manifest = serde_json::from_str(&contents).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("manifesto de runtimes inválido: {error}"),
            )
        })?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let mut runtimes = HashMap::with_capacity(manifest.runtimes.len());
        let mut defaults = HashSet::new();

        for entry in manifest.runtimes {
            let installed = InstalledRuntime::verify(entry, parent)?;
            if installed.is_default && !defaults.insert(installed.runtime.clone()) {
                return Err(invalid_data(format!(
                    "mais de um build {} está marcado como padrão",
                    installed.runtime
                )));
            }
            let id = installed.id.clone();
            if runtimes.insert(id.clone(), installed).is_some() {
                return Err(invalid_data(format!(
                    "o manifesto repete o build de runtime {id}"
                )));
            }
        }

        if runtimes.is_empty() {
            return Err(invalid_data("o manifesto não instala nenhum runtime"));
        }

        Ok(Self {
            runtimes: Arc::new(runtimes),
        })
    }

    pub fn get(&self, id: &str) -> Option<&InstalledRuntime> {
        self.runtimes.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &InstalledRuntime> {
        self.runtimes.values()
    }

    pub fn len(&self) -> usize {
        self.runtimes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.runtimes.is_empty()
    }
}

impl InstalledRuntime {
    fn verify(entry: ManifestRuntime, manifest_parent: &Path) -> io::Result<Self> {
        validate_identifier("id", &entry.id)?;
        validate_identifier("runtime", &entry.runtime)?;
        validate_git_commit(&entry.git_commit)?;
        validate_extension(&entry.source_extension)?;
        if !matches!(
            entry.status.as_str(),
            "development" | "active" | "deprecated"
        ) {
            return Err(invalid_data(format!(
                "o build {} possui status inválido {}",
                entry.id, entry.status
            )));
        }
        if entry.is_default && entry.status != "active" {
            return Err(invalid_data(format!(
                "o build padrão {} precisa estar ativo",
                entry.id
            )));
        }
        if entry.protocol_version != sinteract_protocol_version()? {
            return Err(invalid_data(format!(
                "o build {} usa sinteract.v{}, mas este servidor aceita sinteract.v{}",
                entry.id,
                entry.protocol_version,
                sinteract_protocol_version()?
            )));
        }

        let executable = if entry.executable.is_absolute() {
            entry.executable
        } else {
            manifest_parent.join(entry.executable)
        };
        let executable = executable.canonicalize().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "não foi possível localizar o runtime {} em {}: {error}",
                    entry.id,
                    executable.display()
                ),
            )
        })?;
        if !executable.is_file() {
            return Err(invalid_data(format!(
                "o runtime {} não é um arquivo regular",
                executable.display()
            )));
        }

        let expected_sha256 = normalize_sha256(&entry.sha256)?;
        let actual_sha256 = file_sha256(&executable)?;
        if actual_sha256 != expected_sha256 {
            return Err(invalid_data(format!(
                "o SHA-256 do runtime {} não corresponde ao manifesto",
                entry.id
            )));
        }

        if entry.runtime != "sgleam" {
            return Err(invalid_data(format!(
                "o runtime {} ainda não possui verificador de executável",
                entry.runtime
            )));
        }
        verify_sgleam_version(&executable, &entry.display_version, &entry.git_commit)?;

        Ok(Self {
            id: entry.id,
            runtime: entry.runtime,
            display_name: entry.display_name,
            display_version: entry.display_version,
            git_commit: entry.git_commit,
            protocol_version: entry.protocol_version,
            source_extension: entry.source_extension,
            status: entry.status,
            is_default: entry.is_default,
            executable,
            sha256: expected_sha256,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn runtime(&self) -> &str {
        &self.runtime
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn display_version(&self) -> &str {
        &self.display_version
    }

    pub fn git_commit(&self) -> &str {
        &self.git_commit
    }

    pub fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    pub fn source_extension(&self) -> &str {
        &self.source_extension
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn is_default(&self) -> bool {
        self.is_default
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn matches_spec(&self, spec: &RuntimeSpec) -> bool {
        self.id == spec.runtime_build_id()
            && self.runtime == spec.runtime()
            && self.display_version == spec.display_version()
            && spec.git_commit() == Some(self.git_commit.as_str())
            && spec.protocol_version() == Some(i64::from(self.protocol_version))
    }
}

pub fn sinteract_protocol_version() -> io::Result<u32> {
    sinteract::server::SUBPROTOCOL
        .strip_prefix("sinteract.v")
        .and_then(|version| version.parse().ok())
        .ok_or_else(|| invalid_data("a biblioteca Sinteract possui um subprotocolo inválido"))
}

fn verify_sgleam_version(executable: &Path, version: &str, commit: &str) -> io::Result<()> {
    let output = Command::new(executable)
        .arg("--version")
        .env_clear()
        .output()?;
    if !output.status.success() {
        return Err(invalid_data(format!(
            "o executável {} falhou ao informar sua versão",
            executable.display()
        )));
    }
    let reported = String::from_utf8(output.stdout)
        .map_err(|_| invalid_data("a versão informada pelo Sgleam não é UTF-8 válido"))?;
    let short_commit = &commit[..7];
    if !reported.starts_with(&format!("Version: {version} ("))
        || !reported.trim_end().ends_with(&format!(", {short_commit})"))
    {
        return Err(invalid_data(format!(
            "o executável {} não corresponde ao Sgleam {version} ({short_commit})",
            executable.display()
        )));
    }
    Ok(())
}

fn file_sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn normalize_sha256(value: &str) -> io::Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_data("SHA-256 de runtime inválido"));
    }
    Ok(value)
}

fn validate_identifier(field: &str, value: &str) -> io::Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(invalid_data(format!(
            "o campo {field} do manifesto não é um identificador válido"
        )));
    }
    Ok(())
}

fn validate_git_commit(commit: &str) -> io::Result<()> {
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_data("commit de runtime inválido"));
    }
    Ok(())
}

fn validate_extension(extension: &str) -> io::Result<()> {
    let Some(extension) = extension.strip_prefix('.') else {
        return Err(invalid_data("extensão de runtime inválida"));
    };
    if extension.is_empty()
        || extension.len() > 16
        || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(invalid_data("extensão de runtime inválida"));
    }
    Ok(())
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeCatalog, file_sha256, normalize_sha256, validate_extension, validate_git_commit,
        validate_identifier,
    };

    #[test]
    fn validates_manifest_identifiers_and_hashes() {
        assert!(validate_identifier("id", "sgleam-0.9.0-341e33a").is_ok());
        assert!(validate_identifier("id", "../sgleam").is_err());
        assert!(validate_git_commit(&"a".repeat(40)).is_ok());
        assert!(validate_git_commit("341e33a").is_err());
        assert!(validate_extension(".gleam").is_ok());
        assert!(validate_extension("../gleam").is_err());
        assert!(normalize_sha256(&"AB".repeat(32)).is_ok());
        assert!(normalize_sha256("abc").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn loads_and_verifies_more_than_one_installed_build() {
        use std::{fs, os::unix::fs::PermissionsExt};

        let directory = std::env::temp_dir().join(format!(
            "sarcade-runtime-catalog-test-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::create_dir(&directory).unwrap();
        let executable = directory.join("sgleam");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf 'Version: 0.9.0 (test, 341e33a)\\n'\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let sha256 = file_sha256(&executable).unwrap();
        let commit = "341e33a3ebdd673dc74b283be74bbf8f9c6760f1";
        let manifest = directory.join("runtimes.json");
        fs::write(
            &manifest,
            format!(
                r#"{{"runtimes":[
                    {{"id":"sgleam-0.9.0-a","runtime":"sgleam","display_name":"Sgleam","display_version":"0.9.0","git_commit":"{commit}","protocol_version":1,"source_extension":".gleam","status":"active","is_default":true,"executable":"sgleam","sha256":"{sha256}"}},
                    {{"id":"sgleam-0.9.0-b","runtime":"sgleam","display_name":"Sgleam","display_version":"0.9.0","git_commit":"{commit}","protocol_version":1,"source_extension":".gleam","status":"deprecated","executable":"sgleam","sha256":"{sha256}"}}
                ]}}"#
            ),
        )
        .unwrap();

        let catalog = RuntimeCatalog::load(&manifest).unwrap();
        assert_eq!(catalog.len(), 2);
        assert!(catalog.get("sgleam-0.9.0-a").unwrap().is_default());
        assert_eq!(
            catalog.get("sgleam-0.9.0-b").unwrap().status(),
            "deprecated"
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
