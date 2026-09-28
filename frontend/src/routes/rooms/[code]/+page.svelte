<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { SvelteMap, SvelteURLSearchParams } from 'svelte/reactivity';
	import GameControls from '$lib/components/room/GameControls.svelte';
	import GameViewport from '$lib/components/room/GameViewport.svelte';
	import RoomJoinForm from '$lib/components/room/RoomJoinForm.svelte';
	import RoomLobby from '$lib/components/room/RoomLobby.svelte';
	import RoomToolbar from '$lib/components/room/RoomToolbar.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import type { RoomServerMessage, RoomSnapshot } from '$lib/types/room';

	let room: RoomSnapshot | null = null;
	let socket: WebSocket | null = null;

	let nickname = '';
	let hostToken = '';

	let myPlayerId: number | null = null;
	let amHost = false;

	let message = '';
	let error = '';
	let frameSvg = '';

	let keepAliveInterval: ReturnType<typeof setInterval> | null = null;
	const WS_CONNECTING = 0;
	const WS_OPEN = 1;
	const WS_CLOSED = 3;
	let connectionState = WS_CLOSED;
	const pressedKeys = new SvelteMap<string, string>();
	const keysWithBrowserAction = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', ' ']);

	const code = page.params.code ?? '';

	$: isConnected = connectionState === WS_OPEN;
	$: isConnecting = connectionState === WS_CONNECTING;
	$: canShowJoinForm = connectionState === WS_CLOSED;
	$: canStart = Boolean(room?.can_start && amHost && room.status === 'waiting');
	$: keyboardEnabled = Boolean(isConnected && room?.status === 'running');
	$: frameUrl = frameSvg ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(frameSvg)}` : '';

	function getWsUrl() {
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';

		const params = new SvelteURLSearchParams();
		params.set('nickname', nickname);

		if (hostToken) {
			params.set('host_token', hostToken);
		}

		return `${protocol}//${window.location.host}/api/rooms/${code}/ws?${params.toString()}`;
	}

	function connect() {
		error = '';
		message = '';

		if (!nickname.trim()) {
			error = 'Informe um nickname.';
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
			message = 'Conectado à sala.';
			startKeepAlive();
		};

		nextSocket.onmessage = (event) => {
			if (socket !== nextSocket) return;

			let data: RoomServerMessage;

			try {
				data = JSON.parse(event.data);
			} catch (err) {
				error = 'O servidor enviou uma mensagem inválida.';
				console.error(err);
				return;
			}

			handleServerMessage(data);
		};

		nextSocket.onclose = () => {
			if (socket !== nextSocket) return;

			socket = null;
			connectionState = WS_CLOSED;
			resetParticipationState();
			message = 'Conexão encerrada.';
			stopKeepAlive();
		};

		nextSocket.onerror = () => {
			if (socket !== nextSocket) return;

			error = 'Erro na conexão WebSocket.';
		};
	}

	function handleServerMessage(data: RoomServerMessage) {
		switch (data.type) {
			case 'joined':
				myPlayerId = data.player_id;
				amHost = data.is_host;
				room = data.room;
				break;

			case 'room_state':
				room = data.room;
				refreshHostStatus();
				break;

			case 'room_started':
				room = data.room;
				refreshHostStatus();
				message = 'Partida iniciada.';
				break;

			case 'host_changed':
				room = data.room;
				refreshHostStatus();
				message =
					data.host_player_id === myPlayerId
						? 'Você agora é o host da sala.'
						: 'O host da sala foi alterado.';
				break;

			case 'player_kicked':
				room = data.room;

				if (data.player_id === myPlayerId) {
					closeSocket();
					error = 'Você foi removido da sala pelo host.';
					return;
				}

				refreshHostStatus();
				break;

			case 'player_left':
				room = data.room;
				refreshHostStatus();
				break;

			case 'input':
				// O evento completo já foi incluído no log por socket.onmessage.
				break;

			case 'frame':
				frameSvg = data.svg;
				break;

			case 'error':
				error = data.message;
				break;

			case 'pong':
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
			error = 'Você não está conectado à sala.';
			return;
		}

		socket.send(JSON.stringify(payload));
	}

	function sendStart() {
		sendMessage({ type: 'start' });
	}

	function sendKick(playerId: number) {
		sendMessage({
			type: 'kick',
			player_id: playerId
		});
	}

	function sendPromote(playerId: number) {
		sendMessage({
			type: 'promote',
			player_id: playerId
		});
	}

	function sendKey(key: string) {
		sendMessage({
			type: 'keydown',
			key
		});
	}

	function sendKeyUp(key: string) {
		sendMessage({
			type: 'keyup',
			key
		});
	}

	function sendPadDown(input: string) {
		sendMessage({
			type: 'paddown',
			input
		});
	}

	function sendPadUp(input: string) {
		sendMessage({
			type: 'padup',
			input
		});
	}

	function isEditableTarget(target: EventTarget | null) {
		if (!(target instanceof HTMLElement)) {
			return false;
		}

		return (
			target.isContentEditable || target.closest('input, textarea, select, button, a') !== null
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
				socket.send(JSON.stringify({ type: 'keyup', key }));
			}
		}

		pressedKeys.clear();
	}

	function startKeepAlive() {
		stopKeepAlive();

		keepAliveInterval = setInterval(() => {
			if (socket?.readyState === WS_OPEN) {
				socket.send(JSON.stringify({ type: 'ping' }));
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

		resetParticipationState();
	}

	function resetParticipationState() {
		myPlayerId = null;
		amHost = false;
		room = null;
		frameSvg = '';
		pressedKeys.clear();
	}

	async function leaveRoom() {
		const gameId = room?.game_id;

		closeSocket();
		nickname = '';
		hostToken = '';
		message = '';
		error = '';

		if (gameId) {
			await goto(resolve('/games/[id]', { id: String(gameId) }), {
				replaceState: true
			});
		} else {
			await goto(resolve('/catalog'), { replaceState: true });
		}
	}

	function copyRoomUrl() {
		const url = new URL(resolve('/rooms/[code]', { code }), window.location.origin).toString();

		navigator.clipboard
			.writeText(url)
			.then(() => {
				message = 'Link da sala copiado.';
			})
			.catch(() => {
				error = 'Não foi possível copiar o link.';
			});
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeyDown);
		window.addEventListener('keyup', handleKeyUp);
		window.addEventListener('blur', releasePressedKeys);

		const params = new SvelteURLSearchParams(window.location.search);

		nickname = params.get('nickname') ?? '';
		hostToken = params.get('host_token') ?? '';

		if (nickname) {
			connect();
		}

		return () => {
			window.removeEventListener('keydown', handleKeyDown);
			window.removeEventListener('keyup', handleKeyUp);
			window.removeEventListener('blur', releasePressedKeys);
			closeSocket();
		};
	});
</script>

<svelte:head>
	<title>Sala {code} — Sarcade</title>
	<meta name="description" content="Lobby e execução de uma partida multiplayer no Sarcade." />
</svelte:head>

<main class="page-shell max-w-5xl">
	<RoomToolbar {code} {isConnected} onCopy={copyRoomUrl} onLeave={() => void leaveRoom()} />

	<div class="mt-7 space-y-5">
		{#if error}
			<Alert tone="error" message={error} />
		{/if}

		{#if message}
			<Alert tone="info" {message} />
		{/if}

		{#if canShowJoinForm}
			<RoomJoinForm bind:nickname {isConnecting} onJoin={connect} />
		{/if}

		{#if room?.status === 'running'}
			<div
				class="mx-auto flex min-h-[calc(100vh-13rem)] w-full max-w-4xl flex-col justify-center gap-5"
			>
				<div class="relative">
					<GameViewport {frameUrl} />
					<GameControls enabled={keyboardEnabled} onPadDown={sendPadDown} onPadUp={sendPadUp} />
				</div>
			</div>
		{/if}

		{#if room}
			<RoomLobby
				{room}
				{myPlayerId}
				{amHost}
				{canStart}
				onStart={sendStart}
				onKick={sendKick}
				onPromote={sendPromote}
			/>
		{/if}
	</div>
</main>
