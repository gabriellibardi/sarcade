# Integração Sarcade ↔ runtime

O Sarcade executa partidas somente por sessões Sinteract. Não existe runtime
reserva: se o build selecionado não possuir um executor disponível, a criação
da sala falha de forma explícita. Hoje o executor implementado é o Sgleam;
outros runtimes devem fornecer a mesma fronteira antes de serem marcados como
disponíveis no cadastro de builds.

## Seleção e preparação

Uma sala fixa um `RuntimeSpec` com a revisão, o build, o código-fonte, o
SHA-256 e os limites de jogadores. Somente builds com estado `development` ou
`active` podem ser publicados ou usados para abrir uma sala. O intervalo
declarado pode conter no máximo 16 jogadores.

O build de release incorpora o executável Sgleam. Na inicialização, o backend
materializa uma cópia privada que pode ser substituída, em desenvolvimento,
por `SGLEAM_EXECUTABLE`. A ausência do executável não muda a semântica da
partida nem ativa uma simulação; ela deixa o runtime indisponível.

Para cada sessão, o backend cria um diretório aleatório com permissão `0700`
em Unix e grava `game.<extensão>` com permissão `0600`. A extensão aceita
somente um identificador ASCII, e o digest do arquivo é comparado ao da
revisão. O caminho materializado é o único argumento entregue ao executor e o
diretório é removido ao encerrar a sessão, inclusive após falhas.

## Sessão Sinteract

O processo recebe `SINTERACT_SESSION=1`. O descritor 3 transporta servidor →
engine e o descritor 4 transporta engine → servidor, conforme o contrato
versionado no submódulo Sinteract. `stdin` e `stdout` são descartados;
`stderr` fica disponível para diagnóstico e nunca transporta mensagens da
sessão.

Uma tarefa Tokio possui o processo, `LobbyCore`, `ServerCore`, os pipes, o
relógio e uma fila limitada a 128 comandos. Nenhum I/O do processo ocorre sob
o mutex global das salas. Partidas e inspeções compartilham um semáforo global,
configurável por `MAX_RUNTIME_PROCESSES` e limitado a 256. A vaga acompanha o
ciclo de vida do processo e é liberada também em falhas. O ciclo é:

1. o processo nasce durante `Preparing`;
2. `Hello` anuncia o intervalo de jogadores;
3. o backend compara esse intervalo ao registrado na revisão e só então muda
   a sala para `Waiting`;
4. `Start` reaproveita o processo e congela a ordem dos participantes;
5. cada conexão recebe um `Conn` que a associa ao jogador autenticado;
6. `leave` libera as entradas da conexão, e uma reconexão invalida o `Conn`
   anterior;
7. ao fechar, o backend encerra o pipe, espera até dois segundos e mata o
   processo se ele não terminar.

O servidor oferece oportunidades de apresentação a 60 Hz e descarta ticks
atrasados. No Sgleam, isso não redefine o relógio lógico de `world`/`mworld`:
`Tick` do `Stage` apresenta a cena mais recente, enquanto `on_tick` evolui no
`tick_rate` declarado pelo jogo, com delta fixo implícito de
`1 / tick_rate`.

## Navegador e WebSocket

O mesmo WebSocket `sarcade.room.v1` usa mensagens JSON textuais para o lobby e
payloads Sinteract binários durante `Running`. O navegador não envia
`player_id`: a conexão e sua geração determinam o assento. O backend entrega
o binário a `ServerCore::from_view` e encaminha a saída de
`ServerCore::next_for` apenas à conexão correspondente.

Mensagens e frames recebidos pelo WebSocket têm limite de 64 KiB. O envio a
um cliente tem timeout de dois segundos; um consumidor atrasado é desconectado
sem bloquear os demais. O estado do jogo permanece privado ao processo, e o
frontend recebe somente assets e quadros definidos pelo Sinteract.

## Inspeção de submissões

A inspeção de código abre uma sessão separada, lê somente o `PlayerRange` do
`Hello` e encerra o processo. Nenhum participante é conectado e nenhum
`Start` é enviado, portanto o laço de `world`/`mworld` não começa. O prazo é de
15 segundos e o diagnóstico capturado de `stderr` é limitado a 32 KiB.

O valor encontrado é mostrado ao autor e inspecionado novamente pelo backend
ao salvar o rascunho. Assim, `min_players` e `max_players` não são aceitos do
formulário.

## Testes

Os testes de ciclo de vida podem iniciar o próprio binário do backend no
subcomando privado `sinteract-test-engine`. Essa fixture implementa a sessão
Sinteract real e nunca é selecionada por uma sala de produção. O teste com a
feature `embedded-sgleam` executa um `mworld` de dois jogadores no executável
incorporado e valida quadros distintos e evolução lógica.
