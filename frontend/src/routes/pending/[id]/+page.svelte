<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";

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

  let reviewMessage = "";
  let actionMessage = "";
  let actionError = "";
  let isReviewing = false;

  async function loadGame() {
    isLoading = true;
    error = "";

    const id = page.params.id;

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

  async function approveGame() {
    if (!game) return;

    isReviewing = true;
    actionMessage = "";
    actionError = "";

    try {
      const response = await fetch(`/api/games/${game.id}/approve`, {
        method: "POST"
      });

      const result = await response.json().catch(() => null);

      if (!response.ok) {
        actionError = result?.message ?? "Erro ao aprovar jogo.";
        return;
      }

      actionMessage = "Jogo aprovado com sucesso.";
      await loadGame();
    } catch (err) {
      actionError = "Falha ao conectar com o servidor.";
      console.error(err);
    } finally {
      isReviewing = false;
    }
  }

  async function rejectGame() {
    if (!game) return;

    if (!reviewMessage.trim()) {
      actionError = "Informe uma mensagem de rejeição.";
      return;
    }

    isReviewing = true;
    actionMessage = "";
    actionError = "";

    try {
      const response = await fetch(`/api/games/${game.id}/reject`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          message: reviewMessage
        })
      });

      const result = await response.json().catch(() => null);

      if (!response.ok) {
        actionError = result?.message ?? "Erro ao rejeitar jogo.";
        return;
      }

      actionMessage = "Jogo rejeitado com sucesso.";
      reviewMessage = "";
      await loadGame();
    } catch (err) {
      actionError = "Falha ao conectar com o servidor.";
      console.error(err);
    } finally {
      isReviewing = false;
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

      {#if game.reviewed_at}
        <p><strong>Revisado em:</strong> {game.reviewed_at}</p>
      {/if}
    </section>

    <section class="mb-6">
      <h2 class="mb-2 text-xl font-semibold">Descrição</h2>
      <p>{game.description}</p>
    </section>

    <section class="mb-6">
      <h2 class="mb-2 text-xl font-semibold">Código-fonte</h2>

      <pre class="overflow-auto rounded border bg-gray-100 p-4 text-sm"><code>{game.source_code}</code></pre>
    </section>

    <section class="mb-6">
      <h2 class="mb-2 text-xl font-semibold">Teste</h2>

      <button type="button" class="rounded border px-4 py-2">
        Testar jogo
      </button>
    </section>

    <section class="mb-6">
      <h2 class="mb-2 text-xl font-semibold">Revisão</h2>

      {#if game.status !== "pending"}
        <p class="mb-3">Este jogo já foi revisado.</p>

        {#if game.review_message}
          <p><strong>Mensagem:</strong> {game.review_message}</p>
        {/if}
      {:else}
        <div class="mb-4">
          <label for="reviewMessage" class="block font-medium">
            Mensagem de rejeição
          </label>

          <textarea
            id="reviewMessage"
            bind:value={reviewMessage}
            rows="3"
            class="mt-1 block w-full rounded border p-2"
            placeholder="Obrigatória apenas para rejeitar"
          />
        </div>

        <div class="flex gap-3">
          <button
            type="button"
            on:click={approveGame}
            disabled={isReviewing}
            class="rounded bg-green-700 px-4 py-2 text-white disabled:opacity-50"
          >
            Aprovar
          </button>

          <button
            type="button"
            on:click={rejectGame}
            disabled={isReviewing}
            class="rounded bg-red-700 px-4 py-2 text-white disabled:opacity-50"
          >
            Rejeitar
          </button>
        </div>
      {/if}

      {#if actionMessage}
        <p class="mt-3 rounded border border-green-300 p-3 text-green-700">
          {actionMessage}
        </p>
      {/if}

      {#if actionError}
        <p class="mt-3 rounded border border-red-300 p-3 text-red-700">
          {actionError}
        </p>
      {/if}
    </section>
  {/if}
</main>
