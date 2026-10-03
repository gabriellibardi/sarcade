# Ciclo de vida de salas e partidas

Este documento registra o contrato do Sarcade durante a migração do runtime
reserva para o Sinteract. O código e os testes continuam sendo a fonte de
verdade. As seções abaixo distinguem o que já está implementado do estado
alvo, para não descrever trabalho futuro como concluído.

## Implementado

### Criação da sala

Ao criar uma sala, o backend seleciona a revisão aprovada mais recente e o
`runtime_build` associado na mesma consulta. Esses dados formam um
`RuntimeSpec` imutável, mantido apenas na memória da sala:

- jogo e revisão aprovada;
- identificador, implementação, versão, commit, versão de protocolo e estado
  do build de runtime;
- extensão e código-fonte aprovado;
- SHA-256 do código-fonte;
- limites mínimo e máximo de jogadores.

Uma aprovação posterior no SQLite não altera uma sala existente. O código e
o token de cada participante não fazem parte de `RoomSnapshot` nem de logs.

O ator do runtime reserva também recebe o `RuntimeSpec` na criação da sala,
mas o subprocesso temporário ainda nasce no `Start`. Isso será substituído
pelo handshake do Sinteract descrito em "Estado alvo".

### Lobby, assentos e reconexão

Enquanto a sala está em `waiting`, uma conexão sem `player_token` cria um
participante. O servidor atribui o `player_id`; o cliente nunca escolhe esse
valor. A capacidade vem do `RuntimeSpec`.

Cada participante recebe um token aleatório de reconexão. O frontend o guarda
em `sessionStorage`, com escopo por sala, e o envia somente para recuperar o
mesmo assento. Nickname não autentica uma reconexão.

Cada conexão de um assento possui uma geração crescente. Ao reconectar:

1. o token recupera o mesmo `player_id` e o mesmo papel de host;
2. a geração anterior recebe `connection_replaced` e é encerrada;
3. qualquer comando atrasado daquela geração é recusado no serviço da sala;
4. a nova conexão recebe o estado atual.

No lobby, uma desconexão remove o participante. Depois do `Start`, a lista é
fixa: uma desconexão apenas marca o participante como desconectado e informa o
runtime para liberar entradas pressionadas. Não é possível entrar com um novo
nickname nem expulsar um assento durante a partida.

### WebSocket atual

O endpoint de sala exige o subprotocolo `sinteract.v1` e limita mensagens e
frames recebidos a 64 KiB, o mesmo limite das mensagens de view do Sinteract.
Ele usa mensagens de texto JSON para lobby, administração, entradas e frames
SVG do runtime reserva. O servidor associa cada ação ao `player_id` e à geração
autenticados pela própria conexão. Frames direcionados só são enviados ao
participante correspondente. O servidor usa Ping/Pong do WebSocket; não há um
segundo keep-alive no protocolo JSON.

Mensagens binárias já são separadas do parser JSON, mas recebem um erro
controlado até o `ServerCore` ser integrado. Portanto, a multiplexação está
negociada, mas o caminho binário ainda não executa entradas do Sinteract.

## Estado alvo com Sinteract

Os estados serão:

```text
Preparing -> Waiting -> Running -> Closing -> Closed
     |          |          |
     +----------+----------+----> Failed
```

- `Preparing`: o processo já nasceu e o servidor aguarda um `Hello` válido;
- `Waiting`: os limites do `Hello` foram validados contra o `RuntimeSpec`;
- `Running`: o host congelou a ordem dos jogadores e o `LobbyCore` tornou-se
  `ServerCore`;
- `Closing`: o servidor fechou a entrada da engine e aguarda sua saída por um
  prazo limitado;
- `Closed` e `Failed`: estados terminais, com causa observável e sem processo
  órfão.

O clique em iniciar não cria outro processo. A ordem congelada no `Start`
define `Player 1..N`, e um mapa interno relaciona esses números aos
`player_id` do Sarcade.

O WebSocket existente continuará único por participante. Texto continuará
reservado ao protocolo administrativo do Sarcade; binário será reservado às
mensagens de view do Sinteract. Quando o `ServerCore` for ligado, o backend
entregará a ele apenas os binários autenticados da conexão correspondente.

O Sarcade escolherá a frequência de ticks. O `ServerCore` impede acúmulo ao
aguardar `tickTaken` antes de aceitar outro tick. A frequência inicial ainda
precisa ser fechada com o orientador e não deve ser codificada como decisão
definitiva antes disso.
