# Segurança do Sarcade

Este documento registra a fronteira de confiança implementada. Ele não
substitui a configuração segura do sistema operacional e do proxy reverso.

## Entradas não confiáveis

São consideradas não confiáveis: código de jogos, thumbnails, metadados,
parâmetros HTTP, cookies, mensagens WebSocket e saída da engine. O backend
valida autenticação e autorização, tamanho, estado da revisão, build de
runtime, extensão, intervalo de jogadores e associação entre conexão e
assento.

Código-fonte é limitado a 200 KiB, thumbnails a 1 MiB e o corpo HTTP a 2 MiB.
Thumbnails aceitam somente assinaturas PNG, JPEG e WebP; SVG e conteúdo
desconhecido não são persistidos por novos envios nem servidos como imagem.
Respostas incluem `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
`Referrer-Policy: same-origin` e uma política que desabilita câmera, microfone
e geolocalização.

## Processos de runtime

Cada inspeção ou sala usa um processo separado e um diretório privado. O
ambiente é limpo, o código materializado é conferido por SHA-256, `stdin` e
`stdout` não transportam a sessão e o processo é encerrado com prazo limitado.
Filas, mensagens e diagnóstico possuem limites.

Um semáforo compartilhado impede criação ilimitada de processos. O padrão é
32 processos simultâneos; `MAX_RUNTIME_PROCESSES` aceita de 1 a 256. O limite
inclui inspeções e partidas e a vaga é devolvida em encerramento, falha ou
remoção da sala. Jogos podem declarar no máximo 16 participantes.

## Autenticação e salas

Sessões usam tokens aleatórios armazenados somente como hash e cookies
`HttpOnly`, `SameSite=Lax` e `Secure` em HTTPS. O OAuth usa state de uso único
e PKCE. O backend verifica papéis em todas as ações administrativas.

Convites privados e tokens de reconexão são aleatórios e não aparecem no
snapshot público. O cliente nunca escolhe seu `player_id`; mensagens binárias
são aplicadas ao assento vinculado à conexão. Tamanho, fase, fila e prazo de
envio do WebSocket são limitados.

## Limitação conhecida e implantação

O subprocesso não é, por si só, uma sandbox de kernel. A superfície disponível
ao jogo é reduzida pela linguagem/runtime, pelo ambiente limpo e pelo protocolo
Sinteract, mas uma falha no executável do runtime continua dentro da conta do
serviço. Em produção, execute o Sarcade como usuário sem privilégios, sem
segredos ou arquivos extras acessíveis, com filesystem e rede restritos pelo
serviço/container e limites de CPU e memória por cgroup. TLS, HSTS e limitação
de requisições por IP devem ficar no proxy reverso, que conhece o endereço real
do cliente.

Uma futura sandbox por processo deve ser adicionada sem conceder ao código do
jogo acesso direto a filesystem, rede ou variáveis do servidor e deve manter
os testes de compatibilidade Sgleam/Sinteract.
