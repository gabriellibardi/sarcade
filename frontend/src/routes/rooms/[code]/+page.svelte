<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount, tick } from 'svelte';
	import { SvelteURLSearchParams } from 'svelte/reactivity';
	import GameControls from '$lib/components/room/GameControls.svelte';
	import GameViewport from '$lib/components/room/GameViewport.svelte';
	import RoomJoinForm from '$lib/components/room/RoomJoinForm.svelte';
	import RoomLobby from '$lib/components/room/RoomLobby.svelte';
	import RoomToolbar from '$lib/components/room/RoomToolbar.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import type { RoomServerMessage, RoomSnapshot } from '$lib/types/room';
	import {
		encodeInput,
		PadButton,
		type PadButton as SinteractPadButton
	} from '$sinteract/protocol';
	import { View as SinteractView, type Link as SinteractLink } from '$sinteract/view';

	let room: RoomSnapshot | null = null;
	let socket: WebSocket | null = null;

	let nickname = '';
	let hostToken = '';
	let playerToken = '';

	let myPlayerId: number | null = null;
	let amHost = false;

	let message = '';
	let error = '';
	let frameSvg = '';
	let gameCanvas: HTMLCanvasElement | null = null;
	let sinteractView: SinteractView | null = null;
	let sinteractLink: SinteractLink | null = null;
	let activatingSinteract = false;

	const WS_CONNECTING = 0;
	const WS_OPEN = 1;
	const WS_CLOSED = 3;
	let connectionState = WS_CLOSED;

	const code = page.params.code ?? '';
	const participationStorageKey = `sarcade:room:${code}:participation`;

	$: isConnected = connectionState === WS_OPEN;
	$: isConnecting = connectionState === WS_CONNECTING;
	$: canShowJoinForm = connectionState === WS_CLOSED;
	$: canStart = Boolean(room?.can_start && amHost && room.status === 'waiting');
	$: frameUrl = frameSvg ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(frameSvg)}` : '';
	$: if (room?.status === 'running' && room.sinteract && gameCanvas && isConnected) {
		void activateSinteract();
	}

	const padButtons: Record<string, SinteractPadButton> = {
		Up: PadButton.DPAD_UP,
		Down: PadButton.DPAD_DOWN,
		Left: PadButton.DPAD_LEFT,
		Right: PadButton.DPAD_RIGHT,
		ActionA: PadButton.A,
		ActionB: PadButton.B
	};

	function getWsUrl() {
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';

		const params = new SvelteURLSearchParams();
		params.set('nickname', nickname);

		if (hostToken) {
			params.set('host_token', hostToken);
		}
		if (playerToken) {
			params.set('player_token', playerToken);
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

		const nextSocket = new WebSocket(getWsUrl(), 'sarcade.room.v1');
		nextSocket.binaryType = 'arraybuffer';
		socket = nextSocket;
		connectionState = WS_CONNECTING;

		nextSocket.onopen = () => {
			if (socket !== nextSocket) return;

			connectionState = WS_OPEN;
			message = 'Conectado à sala.';
		};

		nextSocket.onmessage = (event) => {
			if (socket !== nextSocket) return;

			if (typeof event.data !== 'string') {
				if (event.data instanceof ArrayBuffer && sinteractLink?.onmessage) {
					sinteractLink.onmessage(new Uint8Array(event.data));
					return;
				}

				error = 'O servidor enviou dados binários antes de a view estar pronta.';
				return;
			}

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
				playerToken = data.player_token;
				room = data.room;
				saveParticipation();
				void activateSinteract();
				break;

			case 'connection_replaced':
				clearParticipation();
				closeSocket();
				error = 'Esta conexão foi substituída por uma reconexão.';
				break;

			case 'room_state':
				room = data.room;
				refreshHostStatus();
				break;

			case 'room_started':
				room = data.room;
				refreshHostStatus();
				message = 'Partida iniciada.';
				void activateSinteract();
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
					clearParticipation();
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

			case 'frame':
				frameSvg = data.svg;
				break;

			case 'error':
				if (myPlayerId === null && playerToken) {
					clearParticipation();
				}
				error = data.message;
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

	async function activateSinteract() {
		if (
			activatingSinteract ||
			sinteractView ||
			room?.status !== 'running' ||
			!room.sinteract ||
			!socket ||
			socket.readyState !== WS_OPEN
		) {
			return;
		}

		activatingSinteract = true;
		await tick();

		try {
			const activeSocket = socket;
			if (
				!gameCanvas ||
				!activeSocket ||
				activeSocket.readyState !== WS_OPEN ||
				room?.status !== 'running' ||
				!room.sinteract
			) {
				return;
			}

			let open = true;
			const link: SinteractLink = {
				send(payload) {
					if (open && socket === activeSocket && activeSocket.readyState === WS_OPEN) {
						activeSocket.send(payload);
					}
				},
				close() {
					open = false;
				},
				onopen: null,
				onmessage: null,
				onclose: null
			};
			const view = new SinteractView(gameCanvas, {
				keys: gameCanvas,
				onError(cause) {
					console.error(cause);
					error = 'A view não conseguiu processar uma mensagem do jogo.';
				}
			});

			sinteractLink = link;
			sinteractView = view;
			view.attach(link);
			link.onopen?.();
			gameCanvas.focus();
		} finally {
			activatingSinteract = false;
		}
	}

	function closeSinteract() {
		const view = sinteractView;
		sinteractView = null;
		sinteractLink = null;
		view?.close();
	}

	function sendPad(kind: 'down' | 'up', input: string) {
		const button = padButtons[input];
		if (button === undefined || !sinteractLink) return;

		sinteractLink.send(
			encodeInput({
				kind: 'pad',
				action: { kind, button }
			})
		);
	}

	function closeSocket() {
		const currentSocket = socket;
		socket = null;
		connectionState = WS_CLOSED;
		currentSocket?.close();

		resetParticipationState();
	}

	function resetParticipationState() {
		closeSinteract();
		myPlayerId = null;
		amHost = false;
		room = null;
		frameSvg = '';
	}

	function saveParticipation() {
		if (!playerToken) return;

		sessionStorage.setItem(participationStorageKey, JSON.stringify({ playerToken, nickname }));
	}

	function restoreParticipation() {
		const saved = sessionStorage.getItem(participationStorageKey);
		if (!saved) return;

		try {
			const parsed: unknown = JSON.parse(saved);
			if (
				typeof parsed === 'object' &&
				parsed !== null &&
				'playerToken' in parsed &&
				typeof parsed.playerToken === 'string' &&
				'nickname' in parsed &&
				typeof parsed.nickname === 'string'
			) {
				playerToken = parsed.playerToken;
				if (!nickname) nickname = parsed.nickname;
				return;
			}
		} catch {
			// Um valor local inválido não deve impedir uma nova entrada na sala.
		}

		sessionStorage.removeItem(participationStorageKey);
	}

	function clearParticipation() {
		playerToken = '';
		sessionStorage.removeItem(participationStorageKey);
	}

	async function leaveRoom() {
		const gameId = room?.game_id;

		closeSocket();
		clearParticipation();
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
		const params = new SvelteURLSearchParams(window.location.search);

		nickname = params.get('nickname') ?? '';
		hostToken = params.get('host_token') ?? '';
		restoreParticipation();

		if (nickname) {
			connect();
		}

		return () => {
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
					<GameViewport {frameUrl} sinteract={room.sinteract} bind:canvas={gameCanvas} />
					<GameControls
						enabled={Boolean(sinteractLink)}
						onPadDown={(input) => sendPad('down', input)}
						onPadUp={(input) => sendPad('up', input)}
					/>
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
