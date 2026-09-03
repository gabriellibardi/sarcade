# Protocolo temporário Sarcade ↔ runtime

Este contrato permite integrar o servidor do Sarcade antes de o runtime real
do Sgleam e o formato do Simage estarem disponíveis. Ele é deliberadamente
pequeno: eventos entram no processo por `stdin` e imagens SVG saem por
`stdout`.

## Processo e ciclo de vida

Cada partida iniciada possui um processo de runtime. O backend reserva uma
sessão ao criar a sala, mas só inicia o subprocesso quando recebe o comando
`start`. No protótipo, o backend executa a si próprio como runtime reserva:

```text
backend runtime-stub game-<id>
```

No futuro, esse ponto de criação será substituído por algo equivalente a:

```text
sgleam --server <caminho-do-jogo>
```

Uma tarefa Tokio possui o processo e serializa todas as trocas. Não há uma
thread dedicada por partida. `stdin` e `stdout` são pipes anônimos criados com
`Stdio::piped()`; mensagens de diagnóstico do runtime devem usar somente
`stderr`, pois qualquer texto livre em `stdout` corrompe o protocolo.

## Framing

Cada mensagem, nos dois sentidos, tem este envelope:

```text
+----------------------+-----------------------+
| tamanho: u32 BE      | payload: tamanho bytes|
+----------------------+-----------------------+
```

- O tamanho é um inteiro de quatro bytes em ordem big-endian, compatível com
  `AsyncReadExt::read_u32()` e `AsyncWriteExt::write_u32()`.
- O payload é lido integralmente com `read_exact()`.
- Tamanho zero é inválido.
- O limite atual é 8 MiB, verificado antes da alocação.
- EOF e payload truncado encerram a sessão do runtime.

Esta primeira versão não usa o magic `SIMG` nem a ordem little-endian do
framing atual do Simage. A migração futura deve trocar o codec nos dois lados,
sem alterar a responsabilidade do processo e da tarefa que o controla.

## Servidor → runtime

O payload é JSON UTF-8. Toda mensagem possui `protocol_version: 1` e um campo
`type`.

Início da partida:

```json
{
  "protocol_version": 1,
  "type": "start",
  "game_id": 42,
  "players": [
    { "id": 1, "nickname": "Ana" },
    { "id": 2, "nickname": "Beto" }
  ]
}
```

Entrada de jogador:

```json
{
  "protocol_version": 1,
  "type": "input",
  "player_id": 2,
  "event": { "type": "paddown", "input": "Left" }
}
```

As variantes de `event` atualmente aceitas são `keydown`, `keyup`, `paddown`
e `padup`. O servidor obtém `player_id` da própria conexão WebSocket; o cliente
não escolhe em nome de qual jogador o evento será executado.

Outros comandos:

```json
{ "protocol_version": 1, "type": "player_left", "player_id": 2 }
{ "protocol_version": 1, "type": "tick" }
{ "protocol_version": 1, "type": "stop" }
```

O comando `tick` já está reservado no contrato, mas esta integração inicial
ainda não inicia um relógio de ticks.

## Runtime → servidor

O payload é somente o documento SVG em bytes UTF-8, sem JSON adicional:

```text
[u32 BE tamanho][<svg xmlns="http://www.w3.org/2000/svg" ...>...</svg>]
```

Para cada comando aceito, o runtime retorna exatamente um SVG. Isso torna a
primeira implementação estritamente sequencial: enviar comando, ler resposta,
enviar o próximo comando. O SVG reserva é o mesmo para todos os participantes
e apresenta o histórico recente dos eventos.

O backend valida o limite, o UTF-8 e o início `<svg` antes de publicar:

```json
{ "type": "frame", "svg": "<svg ...>...</svg>" }
```

O navegador substitui a imagem anterior por esse frame. O SVG é usado como
fonte de um elemento `img`, e não inserido como HTML executável.

## Ordenação e backpressure

- A fila é criada por sala e possui um único consumidor.
- `start` é sempre a primeira mensagem do subprocesso.
- Entradas são enfileiradas na ordem em que o backend as aceita.
- A fila de comandos é limitada a 128 mensagens.
- A fila de respostas é limitada a 32 frames.
- Fila cheia ou processo encerrado produz erro controlado; não bloqueia o
  mutex global de salas durante I/O.

O protocolo não contém estado Gleam. Quando o runtime real for conectado, o
estado do jogo permanece privado ao processo; o Sarcade envia eventos e recebe
somente a representação visual.
