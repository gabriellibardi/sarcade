<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";

  type Player = {
    id: number;
    nickname: string;
    is_host: boolean;
  };

  type RoomState = {
    type: "room_state";
    code: string;
    game_id: number;
    status: "Waiting" | "Running";
    min_players: number;
    max_players: number;
    players: Player[];
    can_start: boolean;
  };

  let room: RoomState | null = null;
  let socket: WebSocket | null = null;

  let nickname = "";
  let message = "";
  let error = "";
  let events: string[] = [];

  const code = page.params.code;

  function getWsUrl() {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${window.location.host}/api/rooms/${code}/ws?nickname=${encodeURIComponent(nickname)}`;
  }

  function connect() {
    error = "";
    message = "";

    if (!nickname.trim()) {
      error = "Informe um nickname.";
      return;
    }

    socket = new WebSocket(getWsUrl());

    socket.onopen = () => {
      message = "Conectado à sala.";
    };

    socket.onmessage = (event) => {
      events = [event.data, ...events].slice(0, 20);

      const data = JSON.parse(event.data);

      if (data.type === "room_state") {
        room = data;
      }

      if (data.type === "player_left" || data.type === "room_started") {
        room = data.room;
      }
    };

    socket.onclose = () => {
      message = "Conexão encerrada.";
    };

    socket.onerror = () => {
      error = "Erro na conexão WebSocket.";
    };
  }

  function sendStart() {
    socket?.send(JSON.stringify({ type: "start" }));
  }

  function sendKey(key: string) {
    socket?.send(JSON.stringify({ type: "keydown", key }));
  }

  onMount(() => {
    const params = new URLSearchParams(window.location.search);
    nickname = params.get("nickname") ?? "";
  });
</script>

<main class="mx-auto max-w-3xl p-6">
  <h1 class="mb-6 text-2xl font-bold">Sala {code}</h1>

  {#if !socket || socket.readyState === WebSocket.CLOSED}
    <form on:submit|preventDefault={connect} class="mb-6 space-y-4">
      <div>
        <label for="nickname" class="block font-medium">Nickname</label>
        <input
          id="nickname"
          type="text"
          bind:value={nickname}
          class="block w-full rounded border p-2"
        />
      </div>

      <button type="submit" class="rounded bg-black px-4 py-2 text-white">
        Entrar na sala
      </button>
    </form>
  {/if}

  {#if error}
    <p class="mb-4 rounded border border-red-300 p-3 text-red-700">{error}</p>
  {/if}

  {#if message}
    <p class="mb-4 rounded border p-3">{message}</p>
  {/if}

  {#if room}
    <section class="mb-6 rounded border p-4">
      <h2 class="mb-3 text-xl font-semibold">Estado da sala</h2>

      <p><strong>Jogo:</strong> #{room.game_id}</p>
      <p><strong>Status:</strong> {room.status}</p>
      <p><strong>Jogadores:</strong> {room.players.length} / {room.max_players}</p>
      <p><strong>Mínimo para iniciar:</strong> {room.min_players}</p>

      <h3 class="mt-4 font-semibold">Jogadores conectados</h3>

      <ul class="mt-2 list-disc pl-6">
        {#each room.players as player}
          <li>
            {player.nickname}
            {#if player.is_host}
              <strong>(host)</strong>
            {/if}
          </li>
        {/each}
      </ul>
    </section>

    <section class="mb-6 rounded border p-4">
      <h2 class="mb-3 text-xl font-semibold">Ações de teste</h2>

      <div class="flex flex-wrap gap-2">
        <button
          type="button"
          on:click={sendStart}
          class="rounded border px-3 py-2"
        >
          Iniciar partida
        </button>

        <button type="button" on:click={() => sendKey("ArrowUp")} class="rounded border px-3 py-2">
          ↑
        </button>

        <button type="button" on:click={() => sendKey("ArrowLeft")} class="rounded border px-3 py-2">
          ←
        </button>

        <button type="button" on:click={() => sendKey("ArrowDown")} class="rounded border px-3 py-2">
          ↓
        </button>

        <button type="button" on:click={() => sendKey("ArrowRight")} class="rounded border px-3 py-2">
          →
        </button>
      </div>
    </section>
  {/if}

  <section class="rounded border p-4">
    <h2 class="mb-3 text-xl font-semibold">Eventos recebidos</h2>

    {#if events.length === 0}
      <p>Nenhum evento recebido ainda.</p>
    {:else}
      <pre class="max-h-80 overflow-auto text-sm">{events.join("\n\n")}</pre>
    {/if}
  </section>
</main>
