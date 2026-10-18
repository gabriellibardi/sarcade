#!/bin/sh
set -eu

check_submodule() {
    path="$1"
    recorded="$(git ls-tree HEAD "$path" | awk '{ print $3 }')"

    if [ -z "$recorded" ]; then
        echo "Submódulo $path não está registrado no commit atual" >&2
        exit 1
    fi
    if [ ! -e "$path/.git" ]; then
        echo "Submódulo $path não foi inicializado; execute make setup" >&2
        exit 1
    fi

    actual="$(git -C "$path" rev-parse HEAD)"
    if [ "$actual" != "$recorded" ]; then
        echo "Submódulo $path está em $actual, mas o Sarcade fixa $recorded" >&2
        exit 1
    fi
    if [ -n "$(git -C "$path" status --porcelain --untracked-files=no)" ]; then
        echo "Submódulo $path possui alterações não registradas" >&2
        exit 1
    fi
}

check_submodule sgleam
check_submodule sinteract

locked_sinteract="$(
    sed -n '/^name = "sinteract"$/,/^$/s/^source = .*#\([0-9a-f][0-9a-f]*\)"$/\1/p' \
        sgleam/Cargo.lock
)"
actual_sinteract="$(git -C sinteract rev-parse HEAD)"

if [ "$locked_sinteract" != "$actual_sinteract" ]; then
    echo "O Sgleam fixa Sinteract $locked_sinteract, mas o Sarcade usa $actual_sinteract" >&2
    exit 1
fi

printf 'Runtimes fixados: Sgleam %s, Sinteract %s\n' \
    "$(git -C sgleam rev-parse --short=7 HEAD)" \
    "$(git -C sinteract rev-parse --short=7 HEAD)"
