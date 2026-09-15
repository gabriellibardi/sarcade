<script lang="ts">
  import { onMount } from "svelte";
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

  type InputEvent =
    | {
        type: "keydown" | "keyup";
        key: string;
      }
    | {
        type: "paddown" | "padup";
        input: string;
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
        event: InputEvent;
      }
    | {
        type: "frame";
        svg: string;
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
  let frameSvg = "";

  let keepAliveInterval: ReturnType<typeof setInterval> | null = null;
  const WS_CONNECTING = 0;
  const WS_OPEN = 1;
  const WS_CLOSED = 3;
  let connectionState = WS_CLOSED;
  const pressedKeys = new Map<string, string>();
  const keysWithBrowserAction = new Set([
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    " ",
  ]);

  const code = page.params.code;

  $: isConnected = connectionState === WS_OPEN;
  $: isConnecting = connectionState === WS_CONNECTING;
  $: canShowJoinForm = connectionState === WS_CLOSED;
  $: canStart = Boolean(room?.can_start && amHost && room.status === "waiting");
  $: keyboardEnabled = Boolean(isConnected && room?.status === "running");
  $: frameUrl = frameSvg
    ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(frameSvg)}`
    : "";

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

    nickname = nickname.trim();
    closeSocket();

    const nextSocket = new WebSocket(getWsUrl());
    socket = nextSocket;
    connectionState = WS_CONNECTING;

    nextSocket.onopen = () => {
      if (socket !== nextSocket) return;

      connectionState = WS_OPEN;
      message = "Conectado à sala.";
      startKeepAlive();
    };

    nextSocket.onmessage = (event) => {
      if (socket !== nextSocket) return;

      let data: ServerMessage;

      try {
        data = JSON.parse(event.data);
      } catch (err) {
        addEvent("Mensagem inválida recebida do servidor.");
        console.error(err);
        return;
      }

      if (data.type === "frame") {
        addEvent(`Frame SVG recebido (${data.svg.length} caracteres).`);
      } else {
        addEvent(event.data);
      }

      handleServerMessage(data);
    };

    nextSocket.onclose = () => {
      if (socket !== nextSocket) return;

      socket = null;
      connectionState = WS_CLOSED;
      message = "Conexão encerrada.";
      stopKeepAlive();
      pressedKeys.clear();
    };

    nextSocket.onerror = () => {
      if (socket !== nextSocket) return;

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
        // O evento completo já foi incluído no log por socket.onmessage.
        break;

      case "frame":
        frameSvg = data.svg;
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
    if (!socket || socket.readyState !== WS_OPEN) {
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
      player_id: playerId,
    });
  }

  function sendPromote(playerId: number) {
    sendMessage({
      type: "promote",
      player_id: playerId,
    });
  }

  function sendKey(key: string) {
    sendMessage({
      type: "keydown",
      key,
    });
  }

  function sendKeyUp(key: string) {
    sendMessage({
      type: "keyup",
      key,
    });
  }

  function sendKeyPress(key: string) {
    sendKey(key);
    sendKeyUp(key);
  }

  function sendPad(input: string) {
    sendMessage({
      type: "paddown",
      input,
    });
  }

  function sendPadUp(input: string) {
    sendMessage({
      type: "padup",
      input,
    });
  }

  function sendPadPress(input: string) {
    sendPad(input);
    sendPadUp(input);
  }

  function isEditableTarget(target: EventTarget | null) {
    if (!(target instanceof HTMLElement)) {
      return false;
    }

    return (
      target.isContentEditable ||
      target.closest("input, textarea, select, button, a") !== null
    );
  }

  function keyIdentifier(event: KeyboardEvent) {
    return event.code || event.key;
  }

  function preventBrowserAction(event: KeyboardEvent) {
    if (keysWithBrowserAction.has(event.key)) {
      event.preventDefault();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!keyboardEnabled || isEditableTarget(event.target)) {
      return;
    }

    const identifier = keyIdentifier(event);

    preventBrowserAction(event);

    // Um keydown é enviado por pressionamento. Enquanto a tecla estiver
    // segurada, os eventos automáticos de repetição do navegador são ignorados.
    if (event.repeat || pressedKeys.has(identifier)) {
      return;
    }

    pressedKeys.set(identifier, event.key);
    sendKey(event.key);
  }

  function handleKeyUp(event: KeyboardEvent) {
    const identifier = keyIdentifier(event);
    const pressedKey = pressedKeys.get(identifier);

    if (pressedKey === undefined) {
      return;
    }

    preventBrowserAction(event);
    pressedKeys.delete(identifier);
    sendKeyUp(pressedKey);
  }

  function releasePressedKeys() {
    if (socket?.readyState === WS_OPEN) {
      for (const key of pressedKeys.values()) {
        socket.send(JSON.stringify({ type: "keyup", key }));
      }
    }

    pressedKeys.clear();
  }

  function startKeepAlive() {
    stopKeepAlive();

    keepAliveInterval = setInterval(() => {
      if (socket?.readyState === WS_OPEN) {
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
    releasePressedKeys();
    stopKeepAlive();

    const currentSocket = socket;
    socket = null;
    connectionState = WS_CLOSED;
    currentSocket?.close();

    myPlayerId = null;
    amHost = false;
    frameSvg = "";
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
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    window.addEventListener("blur", releasePressedKeys);

    const params = new URLSearchParams(window.location.search);

    nickname = params.get("nickname") ?? "";
    hostToken = params.get("host_token") ?? "";

    if (nickname) {
      connect();
    }

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      window.removeEventListener("blur", releasePressedKeys);
      closeSocket();
    };
  });
</script>

<main class="mx-auto max-w-4xl p-6">
  <h1 class="mb-2 text-2xl font-bold">Sala {code}</h1>

  <div class="mb-6 flex flex-wrap gap-2">
    <button
      type="button"
      on:click={copyRoomUrl}
      class="rounded border px-3 py-2"
    >
      Copiar link da sala
    </button>

    {#if isConnected}
      <button
        type="button"
        on:click={closeSocket}
        class="rounded border px-3 py-2"
      >
        Sair da sala
      </button>
    {/if}
  </div>

  {#if canShowJoinForm}
    <form
      on:submit|preventDefault={connect}
      class="mb-6 space-y-4 rounded border p-4"
    >
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
      <p>
        <strong>Jogadores:</strong>
        {room.players.length} / {room.max_players}
      </p>
      <p><strong>Mínimo para iniciar:</strong> {room.min_players}</p>
      <p>
        <strong>Você:</strong>
        {myPlayerId ? `Jogador #${myPlayerId}` : "não conectado"}
      </p>
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
      <h2 class="mb-3 text-xl font-semibold">Tela do jogo</h2>

      {#if frameUrl}
        <img
          src={frameUrl}
          alt="Tela atual da partida"
          class="block h-auto w-full rounded border bg-slate-950"
        />
      {:else}
        <p>O runtime ainda não enviou uma imagem.</p>
      {/if}
    </section>

    <section class="mb-6 rounded border p-4">
      <h2 class="mb-3 text-xl font-semibold">Controles do jogo</h2>

      {#if keyboardEnabled}
        <p class="mb-3 text-sm text-green-700">
          Teclado ativo. Pressione uma tecla para enviá-la aos jogadores da
          sala.
        </p>
      {:else}
        <p class="mb-3 text-sm">
          O teclado será ativado quando a partida começar.
        </p>
      {/if}

      <div class="flex flex-wrap gap-2">
        <button
          type="button"
          on:click={() => sendPadPress("Up")}
          disabled={!keyboardEnabled}
          class="rounded border px-3 py-2 disabled:opacity-50"
        >
          Key ↑
        </button>

        <button
          type="button"
          on:click={() => sendPadPress("Left")}
          disabled={!keyboardEnabled}
          class="rounded border px-3 py-2 disabled:opacity-50"
        >
          Key ←
        </button>

        <button
          type="button"
          on:click={() => sendPadPress("Down")}
          disabled={!keyboardEnabled}
          class="rounded border px-3 py-2 disabled:opacity-50"
        >
          Key ↓
        </button>

        <button
          type="button"
          on:click={() => sendPadPress("Right")}
          disabled={!keyboardEnabled}
          class="rounded border px-3 py-2 disabled:opacity-50"
        >
          Key →
        </button>

        <button
          type="button"
          on:click={() => sendPadPress("ActionA")}
          disabled={!keyboardEnabled}
          class="rounded border px-3 py-2 disabled:opacity-50"
        >
          Pad A
        </button>

        <button
          type="button"
          on:click={() => sendPadPress("ActionB")}
          disabled={!keyboardEnabled}
          class="rounded border px-3 py-2 disabled:opacity-50"
        >
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
