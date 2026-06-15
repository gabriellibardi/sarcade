<script lang="ts">
  import { onMount } from "svelte";

  type GameSummary = {
    id: number;
    title: string;
    description: string;
    status: string;
    min_players: number;
    max_players: number;
    created_at: string;
    has_thumbnail: boolean;
    author_name: string | null;
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
        error = result?.message ?? "Erro ao carregar jogos pendentes.";
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

  onMount(loadPendingGames);
</script>

<main class="mx-auto max-w-4xl p-6">
  <h1 class="mb-6 text-2xl font-bold">Jogos pendentes</h1>

  {#if isLoading}
    <p>Carregando...</p>
  {:else if error}
    <p class="text-red-700">{error}</p>
  {:else if games.length === 0}
    <p>Nenhum jogo pendente.</p>
  {:else}
    <div class="space-y-4">
      {#each games as game}
        <article class="flex gap-4 rounded border p-4">
          {#if game.has_thumbnail}
            <img
              src={`/api/games/${game.id}/thumbnail`}
              alt={game.title}
              class="h-24 w-24 rounded border object-cover"
            />
          {:else}
            <div class="flex h-24 w-24 items-center justify-center rounded border text-sm">
              Sem imagem
            </div>
          {/if}

          <div class="flex-1">
            <h2 class="text-xl font-semibold">{game.title}</h2>

            <p class="text-sm">
              Autor: {game.author_name ?? "Sem autor"}
            </p>

            <p class="mt-2">{game.description}</p>

            <p class="mt-2 text-sm">
              Jogadores: {game.min_players} a {game.max_players}
            </p>

            <a
              href={`/pending/${game.id}`}
              class="mt-3 inline-block rounded border px-3 py-2"
            >
              Revisar
            </a>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</main>
