#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    printf 'Uso: %s EXECUTAVEL_SGLEAM DIRETORIO_RELEASE\n' "$0" >&2
    exit 2
fi

source_executable="$1"
release_directory="$2"
sgleam_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' sgleam/cli/Cargo.toml | head -n 1)"
sgleam_commit="$(git -C sgleam rev-parse HEAD)"
short_commit="$(git -C sgleam rev-parse --short=7 HEAD)"
protocol_version="$(sed -n 's/^pub const SUBPROTOCOL: &str = "sinteract\.v\([0-9][0-9]*\)";/\1/p' sinteract/src/wire/framing.rs)"
runtime_id="sgleam-${sgleam_version}-${short_commit}"
runtime_directory="${release_directory}/runtimes"
runtime_executable="${runtime_directory}/${runtime_id}"
manifest="${release_directory}/runtimes.json"

if [ -z "$sgleam_version" ] || [ -z "$protocol_version" ]; then
    printf 'Não foi possível descobrir a versão do Sgleam ou do protocolo Sinteract\n' >&2
    exit 1
fi

mkdir -p "$runtime_directory"
install -m 0755 "$source_executable" "$runtime_executable"
runtime_sha256="$(sha256sum "$runtime_executable" | awk '{ print $1 }')"

printf '%s\n' \
    '{' \
    '  "runtimes": [' \
    '    {' \
    "      \"id\": \"${runtime_id}\"," \
    '      "runtime": "sgleam",' \
    '      "display_name": "Sgleam",' \
    "      \"display_version\": \"${sgleam_version}\"," \
    "      \"git_commit\": \"${sgleam_commit}\"," \
    "      \"protocol_version\": ${protocol_version}," \
    '      "source_extension": ".gleam",' \
    '      "status": "active",' \
    '      "is_default": true,' \
    "      \"executable\": \"runtimes/${runtime_id}\"," \
    "      \"sha256\": \"${runtime_sha256}\"" \
    '    }' \
    '  ]' \
    '}' > "$manifest"

printf 'Runtime %s empacotado em %s\n' "$runtime_id" "$runtime_executable"
printf 'Manifesto gravado em %s\n' "$manifest"
