<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";

  type GameDetails = {
    id: number;
    title: string;
    description: string;
    source_code: string;
    status: string;
    min_players: number;
    max_players: number;
    is_featured: boolean;
    review_message: string | null;
    created_at: string;
    reviewed_at: string | null;
    has_thumbnail: boolean;
    author_name: string | null;
  };

  let game: GameDetails | null = null;
  let isLoading = true;
  let error = "";

  async function loadGame() {
    const id = $page.params.id;

    try {
      const response = await fetch(`/api/games/${id}`);
      const result = await response.json().catch(() => null);

      if (!response.ok) {
        error = result?.message ?? "Erro ao carregar jogo.";
        return;
      }

      game = result;
    } catch (err) {
      error = "Falha ao conectar com o servidor.";
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  onMount(loadGame);
</script>

<main class="mx-auto max-w-4xl p-6">
  <a href="/pending" class="mb-4 inline-block underline">
    ← Voltar
  </a>

  {#if isLoading}
    <p>Carregando...</p>
  {:else if error}
    <p class="text-red-700">{error}</p>
  {:else if game}
    <h1 class="mb-4 text-2xl font-bold">{game.title}</h1>

    {#if game.has_thumbnail}
      <img
        src={`/api/games/${game.id}/thumbnail`}
        alt={game.title}
        class="mb-4 max-h-64 rounded border object-contain"
      />
    {/if}

    <section class="mb-6 space-y-1">
      <p><strong>Status:</strong> {game.status}</p>
      <p><strong>Autor:</strong> {game.author_name ?? "Sem autor"}</p>
      <p><strong>Jogadores:</strong> {game.min_players} a {game.max_players}</p>
      <p><strong>Enviado em:</strong> {game.created_at}</p>
    </section>

    <section class="mb-6">
      <h2 class="mb-2 text-xl font-semibold">Descrição</h2>
      <p>{game.description}</p>
    </section>

    <section class="mb-6">
      <h2 class="mb-2 text-xl font-semibold">Código-fonte</h2>

      <pre class="overflow-auto rounded border bg-gray-100 p-4 text-sm"><code>{game.source_code}</code></pre>
    </section>

    <section class="flex gap-3">
      <button class="rounded border px-4 py-2">
        Testar jogo
      </button>

      <button class="rounded bg-green-700 px-4 py-2 text-white">
        Aprovar
      </button>

      <button class="rounded bg-red-700 px-4 py-2 text-white">
        Rejeitar
      </button>
    </section>
  {/if}
</main>
