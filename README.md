# Sarcade

Arcade para jogos criados com linguagens educacionais e executados por builds
de runtime cadastrados na plataforma.

O contrato entre o servidor e o runtime de partidas está descrito
em [`backend/RUNTIME_PROTOCOL.md`](backend/RUNTIME_PROTOCOL.md).

## Login com Google

O Sarcade usa o fluxo OAuth 2.0 Authorization Code no backend. A aplicação solicita apenas
`openid`, `email` e `profile`; os tokens do Google não são persistidos. Depois do login, o
Sarcade cria sua própria sessão em um cookie `HttpOnly`.

### Configuração local

1. No [Google Cloud Console](https://console.cloud.google.com/auth/clients), crie um cliente OAuth
   do tipo **Aplicativo da Web**.
2. Configure esta URI de redirecionamento autorizada:
   `http://localhost:3000/api/auth/callback`.
3. Copie `backend/.env.example` para `backend/.env`.
4. Preencha `GOOGLE_CLIENT_ID` e `GOOGLE_CLIENT_SECRET` no arquivo local.
5. Em `ADMIN_EMAILS`, informe os e-mails Google dos administradores, separados por vírgulas.

O segredo do cliente nunca deve ser enviado ao Git ou colocado no frontend. O arquivo `.env`
já é ignorado pelo repositório.

Em produção, defina `PUBLIC_BASE_URL` com o endereço HTTPS público. Para a implantação atual:

```env
PUBLIC_BASE_URL=https://sarcade.malbarbo.pro.br
ADMIN_EMAILS=seu-email@gmail.com
```

Cadastre no Google exatamente a mesma URI. O endereço precisa coincidir em protocolo, domínio,
porta e caminho. A porta interna do servidor ou do proxy não deve aparecer no callback público.

Ao iniciar, o backend registra no log o `redirect_uri` escolhido. Se houver credenciais Google,
mas `PUBLIC_BASE_URL` estiver ausente, o login fica desabilitado em vez de usar um endereço
`localhost` silenciosamente.

## Comportamento da autenticação

- Explorar o catálogo, jogar e criar salas continuam públicos.
- Publicar um jogo exige login.
- Novos jogos ficam associados ao usuário autenticado.
- Somente administradores enxergam a seção de revisões e podem consultar, aprovar ou rejeitar
  revisões. O backend também protege diretamente essas rotas.
- A sessão do Sarcade dura 30 dias e pode ser encerrada pelo botão **Sair**.
- Sem as duas credenciais Google, o servidor continua funcionando e informa que o login está
  desabilitado.

## Fluxo público de jogos

- O catálogo e a página inicial levam aos detalhes de cada jogo publicado.
- A criação de sala começa na página do jogo, depois que o participante informa seu nickname.
- O perfil público de um autor mostra seu nome, avatar, data de entrada e somente seus jogos
  aprovados.
- `GET /api/games/{id}` expõe apenas os metadados públicos da versão aprovada. Código-fonte e
  informações da revisão ficam restritos às rotas administrativas de revisão.

## Fluxo do autor e testes privados

- O envio inicial cria um rascunho. O autor pode conferir o código, editar os metadados e testar
  uma partida antes de enviar a versão para revisão.
- O número mínimo e máximo de jogadores não é digitado no formulário. O backend inicia o Sgleam
  somente até o `hello` do Sinteract, lê o intervalo declarado pelo `world`/`mworld` e encerra o
  processo antes de admitir jogadores ou iniciar o laço do jogo. A inspeção é repetida ao salvar,
  portanto o servidor não confia no valor exibido pelo navegador.
- A página **Meus jogos** preserva o histórico de rascunhos e revisões pendentes, aprovadas,
  rejeitadas ou retiradas. Editar uma revisão pendente a retira da fila e cria uma nova versão em
  rascunho; uma rejeição também pode originar uma nova versão sem apagar a justificativa anterior.
- Salas de teste usam exatamente a revisão escolhida, mesmo sem publicação. Elas recebem um token
  de convite aleatório no link; somente o autor, um administrador ou quem recebeu esse link pode
  consultar e entrar na sala.
- Somente builds com executor disponível aparecem como opção de envio e podem
  abrir salas. Nesta versão, o pipeline implementado é o do Sgleam; adicionar
  outra linguagem exige integrar seu executor ao mesmo contrato de runtime,
  sem alterar o fluxo da interface.
- A interface obtém linguagens e versões disponíveis pela API e não fixa nomes
  de engines na navegação ou nas ações. Esses nomes aparecem apenas quando
  identificam a linguagem e a versão escolhidas para um jogo.

A página da sala consulta o servidor antes de pedir o nickname. Assim, convite inválido, sala cheia,
partida já iniciada ou ambiente de execução indisponível são mostrados assim que o link é aberto. A validação é
repetida no WebSocket para cobrir corridas entre a consulta e a entrada.

Se o host sair do lobby, o participante que entrou há mais tempo assume automaticamente e pode
iniciar a partida quando o mínimo de jogadores for alcançado.

## Segurança e limites

- Código, mensagens e uploads são tratados como entrada não confiável. O
  backend repete validações de autoria, estado, runtime e limites sem confiar
  no frontend.
- Partidas e inspeções compartilham um limite de subprocessos. O padrão é 32;
  `MAX_RUNTIME_PROCESSES` aceita valores de 1 a 256. Ao esgotar a capacidade,
  novas operações recebem um erro temporário em vez de criar processos sem
  limite.
- Um jogo pode declarar no máximo 16 participantes. Código-fonte tem limite de
  200 KiB, thumbnail de 1 MiB e o corpo HTTP completo de 2 MiB.
- Thumbnails novas aceitam somente assinaturas PNG, JPEG e WebP. SVG e bytes
  arbitrários são recusados pelo servidor.
- O WebSocket limita mensagens, aplica backpressure e vincula entradas ao
  assento autenticado pela própria conexão. Respostas HTTP recebem cabeçalhos
  contra MIME sniffing, framing e vazamento de referência.

As garantias, a fronteira de confiança e as limitações de isolamento estão em
[`backend/SECURITY.md`](backend/SECURITY.md).

## Ensaio de desempenho RNF08

O cliente `scripts/benchmark-rnf08.ts` reproduz o ensaio local usado na
monografia. Ele mede:

- a ida e volta de `GET /health`;
- o intervalo entre `POST /api/rooms` e o primeiro snapshot `waiting`;
- a chegada do primeiro frame cuja cena confirma a alteração do placar depois
  de uma entrada; e
- a estabilidade, CPU e memória de salas simultâneas com dois participantes.

O ensaio usa o jogo de avaliação **Disputa de pontos**, disponível em
[`scripts/fixtures/rnf08_disputa_pontos.gleam`](scripts/fixtures/rnf08_disputa_pontos.gleam).
A latência termina na chegada do frame ao cliente e não inclui a rasterização
do canvas. Durante a medição de capacidade, cada conexão envia uma ação sem
efeito no placar uma vez por segundo. CPU e memória abrangem o servidor e seus
processos Sgleam.
O percentual de CPU é a média desde o início do processo informada por `ps`,
na qual 100% corresponde a um núcleo lógico.

Inicie uma instância dedicada, identifique o jogo aprovado e o PID do servidor
e execute, em Linux:

```sh
GAME_ID=6 SERVER_PID=12345 \
OUTPUT=/tmp/rnf08-local.json \
deno run --config sinteract/web/deno.json -A scripts/benchmark-rnf08.ts
```

Por padrão são feitas 30 repetições de preparação, 30 repetições de latência e
16 salas por 600 segundos. `BASE_URL`, `PREPARATION_RUNS`, `LATENCY_RUNS`,
`CAPACITY_ROOMS`, `CAPACITY_SECONDS`, `SAMPLE_SECONDS` e `TIMEOUT_MS` permitem
configurar uma execução. O acesso integral do processo Deno é necessário para
consultar a árvore dinâmica em `/proc`, executar `ps`, abrir WebSockets locais
e gravar o resultado; por isso o comando deve ser usado apenas com este script
versionado e em uma máquina de ensaio.

## Administradores

`ADMIN_EMAILS` é a fonte de verdade para a função de administrador. Exemplo com mais de uma
conta:

```env
ADMIN_EMAILS=aluno@gmail.com,orientador@universidade.br
```

Depois de alterar a lista, reinicie o servidor. Na inicialização, usuários presentes na lista são
promovidos e usuários removidos dela perdem o acesso administrativo. Também é possível confirmar
o papel pelo selo **Admin** no cabeçalho ou pelo campo `is_admin: true` retornado por
`GET /api/auth/me`.

## Artefatos de execução

O frontend continua incorporado ao executável Rust. Os runtimes de jogos são
executáveis externos: cada servidor instala um ou mais builds e aponta
`RUNTIME_MANIFEST` para o catálogo que descreve esses artefatos.

### Compilação

O Sinteract e o Sgleam são mantidos como submódulos. Depois de clonar o
Sarcade sem `--recurse-submodules`, inicialize-os antes da primeira compilação:

```sh
git submodule update --init
```

Os gitlinks dos submódulos, `backend/Cargo.lock` e
`frontend/package-lock.json` são parte da entrada versionada do build. O
projeto fixa Rust 1.95.0 em `rust-toolchain.toml`, Node 24.8.0 em `.nvmrc`, usa
Cargo com `--locked` e declara npm 11 no pacote do frontend. `make
verify-sources` também
recusa submódulos ausentes, em commits diferentes ou com alterações locais, e
confere que o Sgleam e o Sarcade usam o mesmo commit do Sinteract.

O manifesto aceita vários builds simultâneos. Para cada um, registra identidade,
versão, commit, protocolo Sinteract, extensão, estado, caminho e SHA-256. Na
inicialização, o backend confere o hash e a saída de `--version`; manifesto
ausente, executável ausente ou metadados divergentes encerram a inicialização.
O caminho pode ser absoluto ou relativo ao diretório do manifesto:

```sh
RUNTIME_MANIFEST=/opt/sarcade/runtimes.json
MAX_RUNTIME_PROCESSES=32
```

Na primeira vez, prepare as dependências e o target MUSL:

```sh
make setup
```

Para compilar somente o frontend e o backend:

```sh
make
```

O binário será criado em
`backend/target/x86_64-unknown-linux-musl/release/backend`.

Para gerar um diretório de release com backend, runtime MUSL e manifesto:

```sh
make release-bundle
RUNTIME_MANIFEST="$PWD/release/runtimes.json" ./release/bin/sarcade
```

O bundle inicial contém o build Sgleam fixado pelo checkout. Outros builds
compatíveis podem ser instalados lado a lado e acrescentados ao array
`runtimes` do manifesto. Cada réplica do Sarcade precisa receber os mesmos
artefatos que pretende executar.

Inspeções e partidas iniciam esse build com `sgleam server game.gleam`. A
sessão usa os descritores 3 e 4 do Sinteract e não depende de uma variável de
ambiente para selecionar o modo servidor.

Use `make check` para executar as verificações do frontend, os testes Rust, o
Clippy e os cenários integrados no Sgleam externo. Estes últimos compilam
programas `world` e `mworld`, inspecionam seus limites e executam uma sessão
multiplayer com perspectivas distintas para dois participantes.

### Identidade do runtime

Cada revisão aponta para uma identidade imutável, no formato recomendado
`sgleam-<versão>-<commit curto>`. Ao iniciar, o backend sincroniza no SQLite os
builds instalados e marca builds ausentes como `unavailable`, sem reatribuir
revisões a uma versão diferente.

Builds `development` e `active` aceitam novas submissões; somente `active` pode
ser padrão. Um build `deprecated` permanece instalado para executar jogos já
publicados, mas não recebe novas submissões. Remova seu executável apenas
quando nenhuma revisão que precise continuar disponível depender dele.

## Versão do TCC e citação

A versão utilizada na monografia está marcada pela tag `tcc-2026`. O arquivo
[`CITATION.cff`](CITATION.cff) contém os metadados para citar o software e a
tag fixa também os commits dos submódulos Sgleam e Sinteract usados no build.

## Licença

O código próprio do Sarcade é distribuído sob a licença MIT. Os submódulos e
demais dependências conservam suas respectivas licenças.
