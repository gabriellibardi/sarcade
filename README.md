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

A página da sala consulta o servidor antes de pedir o nickname. Assim, convite inválido, sala cheia,
partida já iniciada ou runtime indisponível são mostrados assim que o link é aberto. A validação é
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

## Binário autocontido

O frontend continua incorporado ao executável Rust. As bibliotecas do OAuth também são compiladas
no mesmo binário; em execução, são necessárias somente as variáveis de ambiente, o banco SQLite e
acesso HTTPS aos serviços do Google.

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

O build de release compila o Sgleam para MUSL e incorpora o executável ao
binário do Sarcade. Na inicialização, o backend materializa uma cópia privada
desse runtime e a reutiliza em todas as partidas. Assim, o servidor não precisa
de um executável Sgleam instalado à parte.

Em um build de desenvolvimento sem o runtime incorporado, `SGLEAM_EXECUTABLE`
pode indicar um executável externo. A variável também substitui o runtime
incorporado quando está definida. O executável substituto deve corresponder ao
mesmo build Sgleam fixado pelo checkout; a variável não altera a identidade do
runtime registrada nas revisões. Na inicialização, o backend executa
`--version` e recusa uma versão ou commit diferente:

```sh
SGLEAM_EXECUTABLE=/caminho/para/sgleam
MAX_RUNTIME_PROCESSES=32
```

Na primeira vez, prepare as dependências e o target MUSL:

```sh
make setup
```

Nas compilações seguintes, execute na raiz do projeto:

```sh
make
```

Esse comando compila primeiro o frontend e depois executa o build release do backend para
`x86_64-unknown-linux-musl`. O mesmo target produz o Sgleam incorporado, por
isso ambos funcionam em distribuições como o NixOS sem um carregador dinâmico
GNU. O binário autocontido será criado em
`backend/target/x86_64-unknown-linux-musl/release/backend`.

Use `make check` para executar as verificações do frontend, os testes Rust, o
Clippy e os cenários integrados no Sgleam incorporado. Estes últimos compilam
programas `world` e `mworld`, inspecionam seus limites e executam uma sessão
multiplayer com perspectivas distintas para dois participantes.

### Identidade do runtime

Durante a compilação, o backend deriva do checkout uma identidade imutável no
formato `sgleam-<versão>-<commit curto>`, além do commit completo do Sgleam e
da versão do protocolo Sinteract. Esses valores alimentam o cadastro de builds
no SQLite. Cada revisão passa a apontar para essa identidade exata; o antigo
`sgleam-dev` é migrado uma única vez para o build atualmente incorporado e
depois fica indisponível.

Atualizar Sgleam ou Sinteract exige atualizar os gitlinks, gerar uma nova
migração/cadastro de build e validar os jogos antes de tornar o novo build o
padrão. Builds antigos não são silenciosamente executados por um binário novo.
Isso torna as entradas e a compatibilidade do artefato rastreáveis; não é uma
promessa de que dois compiladores/sistemas distintos produzam bytes idênticos.
