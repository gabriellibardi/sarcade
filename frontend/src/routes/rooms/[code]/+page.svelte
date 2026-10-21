<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount, tick } from 'svelte';
	import { SvelteURLSearchParams } from 'svelte/reactivity';
	import { apiRequest, errorMessage } from '$lib/api';
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
	let roomPreview: RoomSnapshot | null = null;
	let socket: WebSocket | null = null;

	let nickname = '';
	let hostToken = '';
	let playerToken = '';
	let inviteToken = '';

	let myPlayerId: number | null = null;
	let amHost = false;

	let message = '';
	let error = '';
	let gameCanvas: HTMLCanvasElement | null = null;
	let sinteractView: SinteractView | null = null;
	let sinteractLink: SinteractLink | null = null;
	let activatingSinteract = false;
	let isCheckingRoom = true;
	let entryBlocked = false;

	const WS_CONNECTING = 0;
	const WS_OPEN = 1;
	const WS_CLOSED = 3;
	let connectionState = WS_CLOSED;

	const code = page.params.code ?? '';
	const participationStorageKey = `sarcade:room:${code}:participation`;

	$: isConnected = connectionState === WS_OPEN;
	$: isConnecting = connectionState === WS_CONNECTING;
	$: canShowJoinForm = connectionState === WS_CLOSED && !isCheckingRoom && !entryBlocked;
	$: canStart = Boolean(room?.can_start && amHost && room.status === 'waiting');
	$: if (room?.status === 'running' && gameCanvas && isConnected) {
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
		if (inviteToken) {
			params.set('invite_token', inviteToken);
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
		if (!playerToken && roomPreview && !roomPreview.can_join) {
			entryBlocked = true;
			error = roomPreview.join_error ?? 'Não é possível entrar nesta sala.';
			return;
		}
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
				if (data.room.status !== 'running') closeSinteract();
				break;

			case 'room_started':
				room = data.room;
				refreshHostStatus();
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
				room?.status !== 'running'
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
		const wasPreview = room?.is_preview;

		closeSocket();
		clearParticipation();
		nickname = '';
		hostToken = '';
		inviteToken = '';
		message = '';
		error = '';

		if (gameId && !wasPreview) {
			await goto(resolve('/games/[id]', { id: String(gameId) }), {
				replaceState: true
			});
		} else {
			await goto(resolve('/catalog'), { replaceState: true });
		}
	}

	function copyRoomUrl() {
		const url = new URL(resolve('/rooms/[code]', { code }), window.location.origin);
		if (inviteToken) url.searchParams.set('invite_token', inviteToken);

		navigator.clipboard
			.writeText(url.toString())
			.then(() => {
				message = 'Link da sala copiado.';
			})
			.catch(() => {
				error = 'Não foi possível copiar o link.';
			});
	}

	async function inspectRoom() {
		isCheckingRoom = true;
		entryBlocked = false;
		error = '';
		const params = new SvelteURLSearchParams();
		if (inviteToken) params.set('invite_token', inviteToken);
		if (playerToken) params.set('player_token', playerToken);
		const query = params.size ? `?${params.toString()}` : '';

		try {
			roomPreview = await apiRequest<RoomSnapshot>(`/api/rooms/${code}${query}`);
			if (!playerToken && !roomPreview.can_join) {
				entryBlocked = true;
				error = roomPreview.join_error ?? 'Não é possível entrar nesta sala.';
			}
		} catch (loadError) {
			entryBlocked = true;
			error = errorMessage(loadError, 'Não foi possível consultar esta sala.');
			console.error(loadError);
		} finally {
			isCheckingRoom = false;
		}
	}

	async function initializeRoom() {
		const params = new SvelteURLSearchParams(window.location.search);

		nickname = params.get('nickname') ?? '';
		hostToken = params.get('host_token') ?? '';
		inviteToken = params.get('invite_token') ?? '';
		restoreParticipation();
		await inspectRoom();

		if (nickname && !entryBlocked) connect();
	}

	onMount(() => {
		void initializeRoom();

		return () => {
			closeSocket();
		};
	});
</script>

<svelte:head>
	<title>Sala {code} — Sarcade</title>
	<meta name="description" content="Lobby e execução de uma partida no Sarcade." />
</svelte:head>

<main class="page-shell max-w-7xl">
	<RoomToolbar {code} {isConnected} onCopy={copyRoomUrl} onLeave={() => void leaveRoom()} />

	<div class="mt-7 space-y-5">
		{#if isCheckingRoom}
			<div
				class="surface-card h-36 animate-pulse"
				aria-label="Consultando disponibilidade da sala"
			></div>
		{/if}
		{#if error}
			<Alert tone="error" message={error} />
		{/if}

		{#if message}
			<Alert tone="info" {message} dismissAfter={5000} onDismiss={() => (message = '')} />
		{/if}

		{#if canShowJoinForm}
			<RoomJoinForm bind:nickname {isConnecting} onJoin={connect} />
		{/if}

		{#if room?.status === 'running'}
			<div class="mx-auto flex min-h-[calc(100vh-11rem)] w-full max-w-6xl flex-col justify-center">
				<div>
					<GameViewport bind:canvas={gameCanvas} />
					<GameControls
						enabled={Boolean(sinteractLink)}
						onPadDown={(input) => sendPad('down', input)}
						onPadUp={(input) => sendPad('up', input)}
					/>
				</div>
			</div>
		{/if}

		{#if room && room.status !== 'running'}
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
