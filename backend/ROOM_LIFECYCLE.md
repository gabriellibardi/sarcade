# Ciclo de vida de salas e partidas

Este documento registra o contrato implementado no Sarcade. O código e os
testes permanecem como fonte de verdade.

## Criação e runtime

Ao criar uma sala pública, o backend seleciona a revisão aprovada mais recente
cujo build de runtime esteja disponível. Uma sala privada de teste fixa a
revisão exata escolhida pelo autor ou por um administrador, inclusive quando
ela ainda é rascunho, está pendente ou foi rejeitada. O `RuntimeSpec` imutável
mantido em memória contém revisão, build, versão, código-fonte, SHA-256 e
limites de jogadores; uma aprovação posterior não altera a sala existente.

Toda sala usa uma sessão Sinteract. O processo nasce durante `Preparing`, e a
sala só muda para `Waiting` depois que um `Hello` válido declarar exatamente
os limites registrados na revisão. Não há runtime reserva. Build indisponível,
executor ausente, protocolo incompatível ou falha de inicialização impedem a
criação ou levam a sala a `Failed`.

Partidas e inspeções competem pelo mesmo limite de subprocessos. Uma sala
mantém sua reserva enquanto o processo está ativo e a libera ao terminar,
falhar ou ser removida. Se a capacidade estiver esgotada, o servidor recusa a
criação antes de executar código. Uma engine que encerra inesperadamente muda
a sala para `Failed`; ela não permanece falsamente em `Running`.

O backend materializa o código em um diretório privado por sessão e nunca usa
um nome ou caminho fornecido pelo autor no comando. A inspeção de uma
submissão usa outra sessão, limitada ao `Hello`, sem admitir jogadores nem
iniciar o laço do jogo.

## Estados

Os estados expostos são:

```text
Preparing ──Hello válido──> Waiting ──Start──> Running
    │                           │                 │
    └──────────── falha ────────┴─────────────────┘──> Failed
```

O encerramento e a remoção são operações internas, não estados expostos no
snapshot. O clique em iniciar reaproveita o processo preparado; não cria outro
runtime. A ordem congelada no `Start` define `Player 1..N`, e um mapa interno
relaciona essas posições aos `player_id` estáveis do Sarcade.

## Convite, lobby e reconexão

Salas privadas recebem um token de convite aleatório, exigido na consulta HTTP
e na primeira conexão WebSocket. Depois da entrada, o token individual de
reconexão autoriza apenas a recuperação do próprio assento.

Antes de mostrar o formulário, o frontend consulta `can_join` e `join_error`.
Convite inválido, sala cheia, partida iniciada ou runtime indisponível aparecem
ao abrir o link. O WebSocket repete todas as validações para cobrir mudanças
entre a consulta e a entrada.

No lobby, uma nova conexão cria o participante e recebe do servidor um
`player_id`, um token de reconexão e uma geração. Ao reconectar:

1. o token recupera o mesmo assento e papel de host;
2. a geração anterior recebe `connection_replaced` e é encerrada;
3. comandos atrasados daquela geração são recusados;
4. a nova conexão recebe o estado atual.

Se o host sair antes do `Start`, o participante que entrou primeiro é
promovido. Os `player_id` são crescentes, tornando a escolha determinística.
Se ninguém restar, a sala e seu runtime são removidos.

Depois do `Start`, os assentos ficam fixos. Uma queda apenas marca o jogador
como desconectado e chama `leave` para liberar entradas pressionadas; não é
possível ocupar aquela vaga com outro nickname. Se todos desconectarem, começa
uma janela de dez minutos. Uma reconexão cancela a expiração; caso contrário,
a partida é encerrada.

## WebSocket da sala

O endpoint exige o subprotocolo `sarcade.room.v1` e limita mensagens e frames
recebidos a 64 KiB. Antes de `Running`, o cliente envia por texto apenas
`start`, `kick` e `promote`; binários são recusados. Durante `Running`, texto do
cliente é recusado e binários são payloads Sinteract completos, sem envelope
adicional do Sarcade. Ping/Pong nativo mantém a conexão viva.

O backend associa cada ação ao jogador e à geração autenticados pela conexão.
`ServerCore::from_view` recebe as entradas, e `ServerCore::next_for` entrega
assets e quadros somente ao destinatário correto. O frontend usa a view do
Sinteract sobre o canvas, sem conhecer a linguagem que implementa o jogo.

O envio possui timeout de dois segundos. Cliente lento ou receptor que perdeu
mensagens é desconectado sem bloquear outros participantes. Uma sala vazia em
`Preparing` ou `Waiting` expira após dez minutos e encerra o processo.

## Ticks

O Sarcade oferece ao `ServerCore` oportunidades de apresentação a 60 Hz, com
política de descartar atrasos. O Sinteract impede que uma view escolha outro
jogador e coordena o fluxo de quadros.

Em jogos Sgleam, `world` e `mworld` conservam seu próprio relógio lógico no
`tick_rate` do jogo. O `Tick` recebido pelo `Stage` apresenta a cena mais
recente e não chama `on_tick` diretamente. Cada passo lógico tem delta fixo
implícito de `1 / tick_rate`; atrasos grandes não geram rajadas de recuperação.
