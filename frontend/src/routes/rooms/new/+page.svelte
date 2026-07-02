<script lang="ts">
  import { goto } from "$app/navigation";
  
  let gameId = 1;
  let nickname = "";

  let isCreating = false;
  let error = "";

  async function createRoom() {
    isCreating = true;
    error = "";

    try {
      const response = await fetch("/api/rooms", {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          game_id: gameId,
          nickname
        })
      });

      const result = await response.json().catch(() => null);

      if (!response.ok) {
        error = result?.message ?? "Erro ao criar sala.";
        return;
      }

      await goto(
        `/rooms/${result.code}?nickname=${encodeURIComponent(nickname)}&host_token=${encodeURIComponent(result.host_token)}`
      );
    } catch (err) {
      error = "Falha ao conectar com o servidor.";
      console.error(err);
    } finally {
      isCreating = false;
    }
  }
</script>

<main class="mx-auto max-w-xl p-6">
  <h1 class="mb-6 text-2xl font-bold">Criar sala</h1>

  <form on:submit|preventDefault={createRoom} class="space-y-4">
    <div>
      <label for="gameId" class="block font-medium">ID do jogo</label>
      <input
        id="gameId"
        type="number"
        min="1"
        bind:value={gameId}
        required
        class="block w-full rounded border p-2"
      />
    </div>

    <div>
      <label for="nickname" class="block font-medium">Nickname</label>
      <input
        id="nickname"
        type="text"
        bind:value={nickname}
        required
        class="block w-full rounded border p-2"
      />
    </div>

    <button
      type="submit"
      disabled={isCreating}
      class="rounded bg-black px-4 py-2 text-white disabled:opacity-50"
    >
      {isCreating ? "Criando..." : "Criar sala"}
    </button>
  </form>

  {#if error}
    <p class="mt-4 rounded border border-red-300 p-3 text-red-700">
      {error}
    </p>
  {/if}
</main>
