# Sarcade

Arcade para jogos criados com Student Languages, atualmente Sgleam e Spython.

O contrato temporário entre o servidor e o runtime de partidas está descrito
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
- A detecção automática e as salas de teste estão disponíveis para Sgleam. Builds Spython continuam
  visíveis no cadastro de runtimes, mas esse fluxo retorna um erro explícito enquanto não houver um
  contrato equivalente para descobrir a quantidade de jogadores.

A página da sala consulta o servidor antes de pedir o nickname. Assim, convite inválido, sala cheia,
partida já iniciada ou runtime indisponível são mostrados assim que o link é aberto. A validação é
repetida no WebSocket para cobrir corridas entre a consulta e a entrada.

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

O build de release compila o Sgleam para MUSL e incorpora o executável ao
binário do Sarcade. Na inicialização, o backend materializa uma cópia privada
desse runtime e a reutiliza em todas as partidas. Assim, o servidor não precisa
de um executável Sgleam instalado à parte.

Em um build de desenvolvimento sem o runtime incorporado, `SGLEAM_EXECUTABLE`
pode indicar um executável externo. A variável também substitui o runtime
incorporado quando está definida:

```sh
SGLEAM_EXECUTABLE=/caminho/para/sgleam
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

Use `make check` para executar as verificações do frontend, os testes Rust e o Clippy.
