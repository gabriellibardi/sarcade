# Sarcade

Arcade para jogos criados com Student Languages, atualmente Sgleam e Spython.

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

Em produção, `PUBLIC_BASE_URL` é obrigatório quando `GOOGLE_REDIRECT_URI` não for informado.
Para a implantação atual:

```env
PUBLIC_BASE_URL=https://sarcade.malbarbo.pro.br
ADMIN_EMAILS=seu-email@gmail.com
```

Como alternativa, o callback pode ser informado diretamente:

```env
GOOGLE_REDIRECT_URI=https://sarcade.malbarbo.pro.br/api/auth/callback
```

Cadastre no Google exatamente a mesma URI. O endereço precisa coincidir em protocolo, domínio,
porta e caminho. A porta interna do servidor ou do proxy não deve aparecer no callback público.

Ao iniciar, o backend registra no log o `redirect_uri` escolhido. Se houver credenciais Google,
mas nenhuma das duas variáveis acima, o login fica desabilitado em vez de usar um endereço
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
