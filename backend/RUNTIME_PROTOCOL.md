# Protocolo temporário Sarcade ↔ runtime

Este contrato permite integrar o servidor do Sarcade antes de o runtime real
do Sgleam e o formato do Simage estarem disponíveis. Ele é deliberadamente
pequeno: eventos entram no processo por `stdin` e imagens SVG saem por
`stdout`.

## Processo e ciclo de vida

Cada sala inicia uma sessão e seu subprocesso durante `Preparing`. O runtime
reserva é considerado pronto depois que o processo e os dois pipes foram
criados; ele então espera o comando `start` sem produzir frames. Na migração
para Sinteract, essa confirmação local será substituída pela validação do
`hello` antes de a sala ficar disponível. No protótipo, o backend executa a si
próprio como runtime reserva:

```text
backend runtime-stub game-<id>
```

No futuro, esse ponto de criação será substituído por algo equivalente a:

```text
sgleam --server <caminho-do-jogo>
```

Antes de iniciar o subprocesso, o backend materializa exatamente o código da
revisão fixada no `RuntimeSpec`. Cada sessão recebe um diretório aleatório
próprio, com permissão `0700` em Unix, e um arquivo `game.<extensão>` com
permissão `0600`. A extensão é validada como identificador ASCII e não pode
escolher um caminho. O SHA-256 do arquivo é conferido contra o digest da
revisão aprovada. O diretório é removido quando a sessão termina, inclusive em
falhas de inicialização.

O runtime reserva ainda não lê esse arquivo: a materialização já valida a
fronteira que será consumida pelo launcher do Sgleam, enquanto o subprocesso
temporário continua apenas mostrando o histórico de eventos.

Uma tarefa Tokio possui o processo e escreve os comandos. Uma tarefa auxiliar
mantém a leitura contínua de `stdout`; não há uma thread dedicada por partida.
`stdin` e `stdout` são pipes anônimos criados com `Stdio::piped()`; mensagens de
diagnóstico do runtime devem usar somente `stderr`, pois qualquer texto livre
em `stdout` corrompe o protocolo.

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

Este protocolo temporário não usa o magic `SIMG` nem a ordem little-endian do
framing atual do Simage. A migração futura deve trocar o codec nos dois lados,
sem alterar a responsabilidade do processo e da tarefa que o controla.

## Servidor → runtime

O payload é JSON UTF-8. Toda mensagem possui `protocol_version: 2` e um campo
`type`.

Início da partida:

```json
{
  "protocol_version": 2,
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
  "protocol_version": 2,
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
{ "protocol_version": 2, "type": "player_left", "player_id": 2 }
{ "protocol_version": 2, "type": "tick" }
{ "protocol_version": 2, "type": "stop" }
```

Depois de `start`, a lista de jogadores é fixa. `player_left` informa a queda
da conexão e permite ao runtime liberar entradas pressionadas, mas não remove
o assento: o mesmo jogador pode voltar com seu token de reconexão.

O comando `tick` já está reservado no contrato, mas esta integração inicial
ainda não inicia um relógio de ticks.

## Runtime → servidor

Cada payload de saída é um frame visual direcionado:

```text
+------------------+----------------+---------------------+------------------+
| versão: u32 BE   | tipo: u8       | player_id: u64 BE   | SVG: bytes       |
| 2                | 1 = frame      | destinatário        | restante         |
+------------------+----------------+---------------------+------------------+
```

O `u32` externo do framing contém o tamanho de todo esse payload: 13 bytes de
cabeçalho mais o SVG. O SVG permanece em bytes UTF-8 brutos; ele não contém nem
precisa incorporar metadados de roteamento.

`player_id` é o identificador estável da sessão atribuído pelo Sarcade e
informado ao runtime no comando `start`. O runtime não pode escolher um usuário
global ou um participante de outra sala.

Não existe relação obrigatória de uma resposta por comando. Uma mudança de
estado pode produzir zero ou vários frames, normalmente um para cada jogador
ativo, e o runtime pode produzir frames por causa de ticks. O backend mantém
uma leitura contínua de `stdout`, preserva a ordem do pipe e encaminha cada
frame somente ao WebSocket cujo `player_id` corresponde ao destinatário.

O backend valida versão, tipo, tamanho, UTF-8 e o início `<svg` antes de
encaminhar ao cliente correto:

```json
{ "type": "frame", "svg": "<svg ...>...</svg>" }
```

O navegador substitui sua imagem anterior por esse frame. O SVG é usado como
fonte de um elemento `img`, e não inserido como HTML executável.

## Ordenação e backpressure

- A fila é criada por sala e possui um único consumidor.
- `start` é sempre a primeira mensagem do subprocesso.
- Entradas são enfileiradas na ordem em que o backend as aceita.
- A fila de comandos é limitada a 128 mensagens.
- A fila de saída é limitada a 32 frames direcionados.
- Fila cheia ou processo encerrado produz erro controlado; não bloqueia o
  mutex global de salas durante I/O.
- A tarefa leitora completa cada `read_u32`/`read_exact` sem cancelá-lo no meio;
  comandos novos não interrompem uma leitura parcialmente iniciada.

O processo recebe um ambiente vazio e usa o diretório materializado como
diretório de trabalho. `stderr` continua herdado pelo backend para diagnóstico.
Depois de `stop` ou do fechamento da fila, o runtime tem dois segundos para
encerrar e fechar sua saída; após esse prazo o backend o mata e coleta. Queda
antes de `stop`, segundo `start` e eventos anteriores ao primeiro `start`
encerram a sessão com falha controlada.

O protocolo não contém estado Gleam. Quando o runtime real for conectado, o
estado do jogo permanece privado ao processo; o Sarcade envia eventos e recebe
somente a representação visual.
