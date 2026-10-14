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

O ator do runtime reserva recebe o `RuntimeSpec` e inicia o subprocesso na
criação da sala. A sala começa em `Preparing` e passa a `Waiting` quando o
processo e os pipes estão disponíveis. O `Start` envia somente o primeiro
comando ao processo que já existe; não relança nem rematerializa a fonte. No
Sinteract, a prontidão local será substituída pelo handshake descrito em
"Estado alvo".

Builds marcados como `unavailable` e versões explícitas de protocolo
incompatíveis levam a sala a `Failed` antes do `Start`. Builds de
desenvolvimento antigos, cuja versão é `NULL`, continuam aceitos durante a
migração.

Ao lançar um runtime, o backend cria um diretório privado e exclusivo,
materializa `game.<extensão>` e confere seu SHA-256 contra o `RuntimeSpec`.
Nenhum nome ou caminho fornecido pelo autor entra diretamente no comando. O
diretório permanece vivo pela duração do subprocesso e é removido ao final.

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

O endpoint de sala exige o subprotocolo `sarcade.room.v1` e limita mensagens e
frames recebidos a 64 KiB, o mesmo limite das mensagens de view do Sinteract.
O protocolo da conexão reserva texto ao JSON administrativo do Sarcade e
binário a um payload completo Sinteract v1, sem envelope adicional. O nome do
subprotocolo é do Sarcade porque a conexão não é uma sessão Sinteract pura.

Antes de `Running`, o cliente pode enviar por texto somente `start`, `kick` e
`promote`; binários são recusados. O `Start` mantém aberto o mesmo WebSocket e
muda sua fase. Em `Running`, todo texto enviado pelo cliente é recusado e a
conexão é encerrada; `kick`, `promote`, um segundo `start`, entradas JSON e ping
JSON não pertencem à partida. O keep-alive usa Ping/Pong nativo do WebSocket.
O servidor ainda pode usar texto para comunicar falha controlada.

Mensagens binárias já são separadas do parser JSON, mas recebem um erro
controlado e encerram a conexão até o `ServerCore` ser integrado. Durante a
migração, frames SVG do runtime reserva ainda são mensagens textuais
direcionadas ao participante correspondente. Portanto, o contrato de fases e a
multiplexação já estão implementados, mas o caminho binário ainda não executa
entradas nem frames do Sinteract.

O servidor associa cada ação ao `player_id` e à geração autenticados pela
própria conexão. Reconectar com o token substitui a geração anterior. Em
`Running`, o futuro `ServerCore::Conn` acrescentará a geração específica do
Sinteract sem permitir que a view escolha seu jogador.

O envio para um cliente tem timeout de dois segundos. Se o receptor do canal
da sala ficar para trás e perder mensagens, a conexão é encerrada e o número
descartado é registrado com sala e jogador; ela nunca bloqueia os outros
participantes. O backend também registra o tempo de preparação do processo e,
sem incluir código ou tokens, o intervalo entre `Start` e o primeiro frame.

Uma sala que continua vazia por dez minutos é removida por uma tarefa própria,
mesmo que não chegue outra requisição para disparar limpeza oportunista. A
remoção envia `stop`, fecha a fila e libera processo e diretório temporário.
Depois do `Start`, os assentos continuam reservados durante quedas; quando o
último participante desconecta, começa uma janela de reconexão de dez minutos.
Qualquer reconexão cancela aquela expiração. Se ninguém voltar, a partida e o
runtime são encerrados.

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
reservado ao protocolo administrativo enviado pelo servidor; depois de
`Running`, o cliente enviará somente binário. Quando o `ServerCore` for ligado,
o backend entregará a ele apenas os binários autenticados da conexão
correspondente. O frontend usará `View.attach(Link)` para entregar binários à
view sem tirar do Sarcade a propriedade do socket e das mensagens textuais.

O Sarcade escolherá a frequência de ticks. O `ServerCore` impede acúmulo ao
aguardar `tickTaken` antes de aceitar outro tick. A frequência inicial ainda
precisa ser fechada com o orientador e não deve ser codificada como decisão
definitiva antes disso.

O ator Sinteract e seu subprocesso de conformidade por fd 3/fd 4 já existem no
backend e validam `Hello`, `Start`, frames individuais, reconexão e
encerramento. Nesta etapa eles permanecem separados da sala pública: conectar o
ator ao `Room` e ao WebSocket é a próxima migração, enquanto o runtime reserva
continua sendo o caminho selecionado pela aplicação.
