<script lang="ts">
  import { onMount } from "svelte";

  type GameSummary = {
    id: number;
    title: string;
    description: string;
    status: string;
    min_players: number;
    max_players: number;
    is_featured: boolean;
    created_at: string;
    has_thumbnail: boolean;
  };

  let games: GameSummary[] = [];
  let isLoading = true;
  let error = "";

  async function loadPendingGames() {
    isLoading = true;
    error = "";

    try {
      const response = await fetch("/api/games/pending");
      const result = await response.json().catch(() => null);

      if (!response.ok) {
        error =
          typeof result === "string"
            ? result
            : result?.message ?? "Erro ao carregar jogos pendentes.";
        return;
      }

      games = result;
    } catch (err) {
      error = "Falha ao conectar com o servidor.";
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadPendingGames();
  });
</script>

<main class="mx-auto max-w-3xl p-6">
  <div class="mb-6 flex items-center justify-between">
    <h1 class="text-2xl font-bold">Jogos pendentes</h1>

    <button
      type="button"
      on:click={loadPendingGames}
      class="rounded border px-3 py-2"
    >
      Recarregar
    </button>
  </div>

  {#if isLoading}
    <p>Carregando jogos pendentes...</p>
  {:else if error}
    <div class="rounded border border-red-300 p-3 text-red-700">
      {error}
    </div>
  {:else if games.length === 0}
    <p>Nenhum jogo pendente no momento.</p>
  {:else}
    <div class="space-y-4">
      {#each games as game}
        <article class="rounded border p-4">
          <div class="mb-2 flex items-start justify-between gap-4">
            <div>
              <h2 class="text-xl font-semibold">
                #{game.id} — {game.title}
              </h2>

              <p class="text-sm text-gray-600">
                Enviado em: {game.created_at}
              </p>
            </div>

            <span class="rounded border px-2 py-1 text-sm">
              {game.status}
            </span>
          </div>

          <p class="mb-3">
            {game.description}
          </p>

          <p class="text-sm">
            Jogadores: {game.min_players} a {game.max_players}
          </p>

          <p class="text-sm">
            Thumbnail: {game.has_thumbnail ? "sim" : "não"}
          </p>
        </article>
      {/each}
    </div>
  {/if}
</main>
