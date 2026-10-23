use std::{env, fs, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../sgleam/cli/Cargo.toml");
    println!("cargo:rerun-if-changed=../sinteract/src/wire/framing.rs");
    track_revision("../sgleam");
    track_revision("../sinteract");

    if env::var_os("CARGO_FEATURE_EMBEDDED_SGLEAM").is_some() {
        require_clean("../sgleam");
        require_clean("../sinteract");
        require_recorded("sgleam", "../sgleam");
        require_recorded("sinteract", "../sinteract");
    }

    let sgleam_version = package_version(Path::new("../sgleam/cli/Cargo.toml"));
    let sgleam_commit = revision("../sgleam");
    let sinteract_commit = revision("../sinteract");
    let protocol_version = sinteract_protocol(Path::new("../sinteract/src/wire/framing.rs"));
    let short_commit = sgleam_commit
        .get(..7)
        .expect("o commit do Sgleam deve ter ao menos sete caracteres");

    println!("cargo:rustc-env=SARCADE_SGLEAM_BUILD_ID=sgleam-{sgleam_version}-{short_commit}");
    println!("cargo:rustc-env=SARCADE_SGLEAM_VERSION={sgleam_version}");
    println!("cargo:rustc-env=SARCADE_SGLEAM_GIT_COMMIT={sgleam_commit}");
    println!("cargo:rustc-env=SARCADE_SINTERACT_GIT_COMMIT={sinteract_commit}");
    println!("cargo:rustc-env=SARCADE_SINTERACT_PROTOCOL_VERSION={protocol_version}");
}

fn track_revision(repository: &str) {
    let output = Command::new("git")
        .args(["-C", repository, "rev-parse", "--git-path", "HEAD"])
        .output()
        .unwrap_or_else(|error| panic!("não foi possível localizar HEAD de {repository}: {error}"));
    if output.status.success() {
        let head = String::from_utf8(output.stdout).expect("caminho Git que não é UTF-8");
        println!("cargo:rerun-if-changed={}", head.trim());
    }
}

fn require_clean(repository: &str) {
    let output = Command::new("git")
        .args([
            "-C",
            repository,
            "status",
            "--porcelain",
            "--untracked-files=no",
        ])
        .output()
        .unwrap_or_else(|error| panic!("não foi possível consultar {repository}: {error}"));
    assert!(
        output.status.success() && output.stdout.is_empty(),
        "o build incorporado exige o submódulo {repository} sem alterações locais"
    );
}

fn require_recorded(path: &str, repository: &str) {
    let tree = Command::new("git")
        .args(["-C", "..", "ls-tree", "HEAD", path])
        .output()
        .unwrap_or_else(|error| panic!("não foi possível consultar o gitlink de {path}: {error}"));
    let recorded = String::from_utf8(tree.stdout)
        .expect("gitlink que não é UTF-8")
        .split_whitespace()
        .nth(2)
        .map(str::to_string)
        .unwrap_or_else(|| panic!("submódulo {path} não registrado no Sarcade"));
    let actual = revision(repository);

    assert_eq!(
        actual, recorded,
        "o build incorporado exige {path} no commit registrado pelo Sarcade"
    );
}

fn revision(repository: &str) -> String {
    let output = Command::new("git")
        .args(["-C", repository, "rev-parse", "HEAD"])
        .output()
        .unwrap_or_else(|error| panic!("não foi possível consultar {repository}: {error}"));
    assert!(
        output.status.success(),
        "não foi possível descobrir a revisão de {repository}; inicialize os submódulos"
    );

    String::from_utf8(output.stdout)
        .expect("git retornou um commit que não é UTF-8")
        .trim()
        .to_string()
}

fn package_version(path: &Path) -> String {
    let manifest = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("não foi possível ler {}: {error}", path.display()));
    let package = manifest
        .split_once("[package]")
        .map(|(_, package)| package)
        .expect("manifesto Sgleam sem seção [package]");

    string_value(package, "version").expect("manifesto Sgleam sem versão do pacote")
}

fn sinteract_protocol(path: &Path) -> String {
    let framing = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("não foi possível ler {}: {error}", path.display()));
    let subprotocol = string_value(&framing, "pub const SUBPROTOCOL: &str")
        .expect("Sinteract sem SUBPROTOCOL versionado");

    subprotocol
        .strip_prefix("sinteract.v")
        .filter(|version| version.chars().all(|character| character.is_ascii_digit()))
        .expect("SUBPROTOCOL do Sinteract não segue sinteract.vN")
        .to_string()
}

fn string_value(source: &str, key: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        (candidate.trim() == key).then(|| {
            value
                .trim()
                .trim_end_matches(';')
                .trim_matches('"')
                .to_string()
        })
    })
}
