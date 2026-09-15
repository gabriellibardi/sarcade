# Frontend do Sarcade

Interface SvelteKit do Sarcade. O build usa o adapter estático e é incorporado ao executável Rust
do projeto. Durante o desenvolvimento, o Vite encaminha `/api` e conexões WebSocket ao backend em
`http://127.0.0.1:3000`.

## Organização

```text
src/
├─ lib/
│  ├─ api.ts              # leitura uniforme de respostas e erros HTTP
│  ├─ auth.ts             # estado da sessão do usuário
│  ├─ components/
│  │  ├─ forms/           # controles de formulário reutilizáveis
│  │  ├─ layout/          # cabeçalho e rodapé globais
│  │  ├─ review/          # apresentação do fluxo de revisão
│  │  ├─ room/            # lobby, jogo, controles e diagnóstico da sala
│  │  └─ ui/              # feedback, estados vazios, cabeçalhos e avatar
│  ├─ types/              # contratos recebidos do backend, por domínio
│  └─ utils/              # formatação sem estado
└─ routes/                # composição das páginas e coordenação dos fluxos
```

As páginas devem concentrar carregamento, navegação e coordenação. Apresentações reutilizáveis,
contratos de domínio e funções sem estado pertencem a `lib/`.

## Desenvolvimento

Com o backend iniciado na porta 3000:

```sh
npm run dev
```

## Verificações

```sh
npm run format
npm run lint
npm run check
npm run build
```

O comando `npm run lint` valida tanto o formato do Prettier quanto as regras do ESLint. O comando
`npm run check` executa a análise de tipos e os diagnósticos do Svelte.
