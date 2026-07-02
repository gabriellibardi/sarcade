<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { page } from "$app/state";

  type Player = {
    id: number;
    nickname: string;
    is_host: boolean;
  };

  type RoomStatus = "waiting" | "running";

  type RoomSnapshot = {
    code: string;
    game_id: number;
    status: RoomStatus;
    min_players: number;
    max_players: number;
    players: Player[];
    can_start: boolean;
  };

  type ServerMessage =
    | {
        type: "joined";
        player_id: number;
        is_host: boolean;
        room: RoomSnapshot;
      }
    | {
        type: "room_state";
        room: RoomSnapshot;
      }
    | {
        type: "room_started";
        room: RoomSnapshot;
      }
    | {
        type: "host_changed";
        host_player_id: number;
        room: RoomSnapshot;
      }
    | {
        type: "player_kicked";
        player_id: number;
        room: RoomSnapshot;
      }
    | {
        type: "player_left";
        player_id: number;
        room: RoomSnapshot;
      }
    | {
        type: "input";
        player_id: number;
        event: unknown;
      }
    | {
        type: "error";
        message: string;
      }
    | {
        type: "pong";
      };

  let room: RoomSnapshot | null = null;
  let socket: WebSocket | null = null;

  let nickname = "";
  let hostToken = "";

  let myPlayerId: number | null = null;
  let amHost = false;

  let message = "";
  let error = "";
  let events: string[] = [];

  let keepAliveInterval: ReturnType<typeof setInterval> | null = null;

  const code = page.params.code;

  $: isConnected = socket?.readyState === WebSocket.OPEN;
  $: isConnecting = socket?.readyState === WebSocket.CONNECTING;
  $: canShowJoinForm = !socket || socket.readyState === WebSocket.CLOSED;
  $: canStart = Boolean(room?.can_start && amHost && room.status === "waiting");

  function getWsUrl() {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";

    const params = new URLSearchParams();
    params.set("nickname", nickname);

    if (hostToken) {
      params.set("host_token", hostToken);
    }

    return `${protocol}//${window.location.host}/api/rooms/${code}/ws?${params.toString()}`;
  }

  function connect() {
    error = "";
    message = "";

    if (!nickname.trim()) {
      error = "Informe um nickname.";
      return;
    }

    closeSocket();

    socket = new WebSocket(getWsUrl());

    socket.onopen = () => {
      message = "Conectado à sala.";
      startKeepAlive();
    };

    socket.onmessage = (event) => {
      addEvent(event.data);

      let data: ServerMessage;

      try {
        data = JSON.parse(event.data);
      } catch (err) {
        addEvent("Mensagem inválida recebida do servidor.");
        console.error(err);
        return;
      }

      handleServerMessage(data);
    };

    socket.onclose = () => {
      message = "Conexão encerrada.";
      stopKeepAlive();
    };

    socket.onerror = () => {
      error = "Erro na conexão WebSocket.";
    };
  }

  function handleServerMessage(data: ServerMessage) {
    switch (data.type) {
      case "joined":
        myPlayerId = data.player_id;
        amHost = data.is_host;
        room = data.room;
        break;

      case "room_state":
        room = data.room;
        refreshHostStatus();
        break;

      case "room_started":
        room = data.room;
        refreshHostStatus();
        message = "Partida iniciada.";
        break;

      case "host_changed":
        room = data.room;
        refreshHostStatus();
        message =
          data.host_player_id === myPlayerId
            ? "Você agora é o host da sala."
            : "O host da sala foi alterado.";
        break;

      case "player_kicked":
        room = data.room;

        if (data.player_id === myPlayerId) {
          error = "Você foi removido da sala pelo host.";
          closeSocket();
          return;
        }

        refreshHostStatus();
        break;

      case "player_left":
        room = data.room;
        refreshHostStatus();
        break;

      case "input":
        // Por enquanto, só aparece no log de eventos.
        break;

      case "error":
        error = data.message;
        break;

      case "pong":
        // Keep-alive respondido pelo servidor.
        break;
    }
  }

  function refreshHostStatus() {
    if (!room || myPlayerId === null) {
      amHost = false;
      return;
    }

    const me = room.players.find((player) => player.id === myPlayerId);
    amHost = me?.is_host ?? false;
  }

  function sendMessage(payload: object) {
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      error = "Você não está conectado à sala.";
      return;
    }

    socket.send(JSON.stringify(payload));
  }

  function sendStart() {
    sendMessage({ type: "start" });
  }

  function sendKick(playerId: number) {
    sendMessage({
      type: "kick",
      player_id: playerId
    });
  }

  function sendPromote(playerId: number) {
    sendMessage({
      type: "promote",
      player_id: playerId
    });
  }

  function sendKey(key: string) {
    sendMessage({
      type: "keydown",
      key
    });
  }

  function sendPad(input: string) {
    sendMessage({
      type: "paddown",
      input
    });
  }

  function startKeepAlive() {
    stopKeepAlive();

    keepAliveInterval = setInterval(() => {
      if (socket?.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: "ping" }));
      }
    }, 25_000);
  }

  function stopKeepAlive() {
    if (keepAliveInterval) {
      clearInterval(keepAliveInterval);
      keepAliveInterval = null;
    }
  }

  function closeSocket() {
    stopKeepAlive();

    if (socket) {
      socket.close();
      socket = null;
    }

    myPlayerId = null;
    amHost = false;
  }

  function addEvent(value: string) {
    events = [value, ...events].slice(0, 30);
  }

  function copyRoomUrl() {
    const url = `${window.location.origin}/rooms/${code}`;

    navigator.clipboard
      .writeText(url)
      .then(() => {
        message = "Link da sala copiado.";
      })
      .catch(() => {
        error = "Não foi possível copiar o link.";
      });
  }

  onMount(() => {
    const params = new URLSearchParams(window.location.search);

    nickname = params.get("nickname") ?? "";
    hostToken = params.get("host_token") ?? "";

    if (nickname) {
      connect();
    }
  });

  onDestroy(() => {
    closeSocket();
  });
</script>

<main class="mx-auto max-w-4xl p-6">
  <h1 class="mb-2 text-2xl font-bold">Sala {code}</h1>

  <div class="mb-6 flex flex-wrap gap-2">
    <button type="button" on:click={copyRoomUrl} class="rounded border px-3 py-2">
      Copiar link da sala
    </button>

    {#if isConnected}
      <button type="button" on:click={closeSocket} class="rounded border px-3 py-2">
        Sair da sala
      </button>
    {/if}
  </div>

  {#if canShowJoinForm}
    <form on:submit|preventDefault={connect} class="mb-6 space-y-4 rounded border p-4">
      <h2 class="text-xl font-semibold">Entrar na sala</h2>

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
        Entrar
      </button>
    </form>
  {/if}

  {#if isConnecting}
    <p class="mb-4 rounded border p-3">Conectando...</p>
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
      <p><strong>Você:</strong> {myPlayerId ? `Jogador #${myPlayerId}` : "não conectado"}</p>
      <p><strong>Você é host?</strong> {amHost ? "sim" : "não"}</p>

      <h3 class="mt-4 font-semibold">Jogadores conectados</h3>

      <ul class="mt-2 space-y-2">
        {#each room.players as player}
          <li class="flex flex-wrap items-center gap-2 rounded border p-2">
            <span>
              #{player.id} — {player.nickname}
              {#if player.is_host}
                <strong>(host)</strong>
              {/if}
              {#if player.id === myPlayerId}
                <em>(você)</em>
              {/if}
            </span>

            {#if amHost && player.id !== myPlayerId}
              <button
                type="button"
                on:click={() => sendKick(player.id)}
                class="rounded border px-2 py-1 text-sm"
              >
                Expulsar
              </button>

              <button
                type="button"
                on:click={() => sendPromote(player.id)}
                class="rounded border px-2 py-1 text-sm"
              >
                Promover a host
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    </section>

    <section class="mb-6 rounded border p-4">
      <h2 class="mb-3 text-xl font-semibold">Ações da sala</h2>

      <div class="flex flex-wrap gap-2">
        <button
          type="button"
          on:click={sendStart}
          disabled={!canStart}
          class="rounded border px-3 py-2 disabled:opacity-50"
        >
          Iniciar partida
        </button>
      </div>

      {#if !amHost}
        <p class="mt-2 text-sm">Apenas o host pode iniciar a partida.</p>
      {:else if room.players.length < room.min_players}
        <p class="mt-2 text-sm">
          Ainda faltam jogadores para iniciar a partida.
        </p>
      {/if}
    </section>

    <section class="mb-6 rounded border p-4">
      <h2 class="mb-3 text-xl font-semibold">Inputs de teste</h2>

      <div class="flex flex-wrap gap-2">
        <button type="button" on:click={() => sendKey("ArrowUp")} class="rounded border px-3 py-2">
          Key ↑
        </button>

        <button type="button" on:click={() => sendKey("ArrowLeft")} class="rounded border px-3 py-2">
          Key ←
        </button>

        <button type="button" on:click={() => sendKey("ArrowDown")} class="rounded border px-3 py-2">
          Key ↓
        </button>

        <button type="button" on:click={() => sendKey("ArrowRight")} class="rounded border px-3 py-2">
          Key →
        </button>

        <button type="button" on:click={() => sendPad("ActionA")} class="rounded border px-3 py-2">
          Pad A
        </button>

        <button type="button" on:click={() => sendPad("ActionB")} class="rounded border px-3 py-2">
          Pad B
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