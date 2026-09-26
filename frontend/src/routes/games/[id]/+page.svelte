<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { ApiError, apiRequest, errorMessage } from '$lib/api';
	import { authState } from '$lib/auth';
	import Alert from '$lib/components/ui/Alert.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import type { PublishedGameDetails } from '$lib/types/game';
	import { formatDate, playerCountLabel } from '$lib/utils/format';

	type CreateRoomResponse = { code: string; host_token: string };

	let game: PublishedGameDetails | null = null;
	let nickname = '';
	let nicknameWasEdited = false;
	let isLoading = true;
	let isCreating = false;
	let notFound = false;
	let loadError = '';
	let createError = '';

	const gameId = Number(page.params.id);

	$: playerLabel = game ? playerCountLabel(game.min_players, game.max_players) : '';

	async function loadGame() {
		isLoading = true;
		notFound = false;
		loadError = '';

		if (!Number.isSafeInteger(gameId) || gameId <= 0) {
			notFound = true;
			isLoading = false;
			return;
		}

		try {
			game = await apiRequest<PublishedGameDetails>(`/api/games/${gameId}`);
		} catch (error) {
			if (error instanceof ApiError && error.status === 404) {
				notFound = true;
				return;
			}

			loadError = errorMessage(error, 'Não foi possível carregar este jogo.');
			console.error(error);
		} finally {
			isLoading = false;
		}
	}

	async function createRoom() {
		if (!game || isCreating) return;

		nickname = nickname.trim();
		createError = '';

		if (!nickname) {
			createError = 'Informe como você quer aparecer na sala.';
			return;
		}

		isCreating = true;

		try {
			const result = await apiRequest<CreateRoomResponse>('/api/rooms', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ game_id: game.id, nickname })
			});

			await goto(
				resolve(
					`/rooms/[code]?nickname=${encodeURIComponent(nickname)}&host_token=${encodeURIComponent(result.host_token)}`,
					{ code: result.code }
				)
			);
		} catch (error) {
			createError = errorMessage(error, 'Não foi possível criar a sala.');
			console.error(error);
		} finally {
			isCreating = false;
		}
	}

	onMount(() => {
		void loadGame();

		const unsubscribe = authState.subscribe((state) => {
			if (!nicknameWasEdited && !nickname && state.user?.name) {
				nickname = state.user.name.slice(0, 30);
			}
		});

		return unsubscribe;
	});
</script>

<svelte:head>
	<title>{game ? `${game.title} — Sarcade` : 'Detalhes do jogo — Sarcade'}</title>
	<meta
		name="description"
		content={game?.description ?? 'Veja os detalhes de um jogo publicado no Sarcade.'}
	/>
</svelte:head>

<main class="mx-auto max-w-6xl px-5 py-12 sm:px-6 lg:py-16">
	<a
		href={resolve('/catalog')}
		class="font-mono text-xs font-bold text-indigo-300 uppercase hover:underline"
		>← Voltar ao catálogo</a
	>

	{#if isLoading}
		<div
			class="mt-7 grid animate-pulse gap-8 lg:grid-cols-[1.3fr_0.7fr]"
			aria-label="Carregando jogo"
		>
			<div class="h-[34rem] border-2 border-arcade-line bg-arcade-panel"></div>
			<div class="h-80 border-2 border-arcade-line bg-arcade-panel"></div>
		</div>
	{:else if notFound}
		<div class="mt-7">
			<EmptyState
				title="Jogo não encontrado"
				description="Ele pode não existir ou ainda não estar aprovado."
			>
				<a href={resolve('/catalog')} class="primary-button">Explorar jogos publicados</a>
			</EmptyState>
		</div>
	{:else if loadError}
		<div class="mt-7"><Alert tone="error" message={loadError} onRetry={loadGame} /></div>
	{:else if game}
		<div class="mt-7 grid items-start gap-8 lg:grid-cols-[1.3fr_0.7fr]">
			<article class="surface-card overflow-hidden">
				<div class="aspect-[16/9] bg-[linear-gradient(135deg,#25181c_0_50%,#191c2d_50%)]">
					{#if game.has_thumbnail}
						<img
							src={`/api/games/${game.id}/thumbnail`}
							alt={`Capa do jogo ${game.title}`}
							class="h-full w-full object-cover"
						/>
					{:else}
						<div class="flex h-full items-center justify-center">
							<span
								class="display-title border-2 border-slate-300 bg-arcade-bg px-10 py-6 text-7xl text-arcade-ink shadow-[7px_7px_0_#3d1a62]"
							>
								{game.title.slice(0, 1).toUpperCase()}
							</span>
						</div>
					{/if}
				</div>

				<div class="p-6 sm:p-8">
					<div class="flex flex-wrap items-center gap-2">
						<span
							class="status-chip bg-indigo-100 px-3 py-1 font-mono text-xs font-bold text-indigo-300"
						>
							{game.runtime_name} · {game.runtime_version}
						</span>
						{#if game.is_featured}
							<span
								class="status-chip bg-amber-300 px-3 py-1 font-mono text-xs font-bold text-amber-950"
								>Destaque</span
							>
						{/if}
					</div>

					<h1 class="display-title mt-5 text-4xl text-slate-950 sm:text-5xl">{game.title}</h1>
					<p class="mt-5 text-lg leading-8 whitespace-pre-line text-slate-600">
						{game.description}
					</p>

					<dl class="mt-8 grid gap-4 border-t-2 border-slate-200 pt-6 sm:grid-cols-3">
						<div>
							<dt class="text-xs font-bold tracking-wide text-slate-400 uppercase">Jogadores</dt>
							<dd class="mt-1 font-bold text-slate-800">{playerLabel}</dd>
						</div>
						<div>
							<dt class="text-xs font-bold tracking-wide text-slate-400 uppercase">Autor</dt>
							<dd class="mt-1 font-bold text-slate-800">
								{#if game.author_id && game.author_name}
									<a
										href={resolve('/users/[id]', { id: String(game.author_id) })}
										class="text-indigo-300 hover:underline">{game.author_name}</a
									>
								{:else}
									Autor não informado
								{/if}
							</dd>
						</div>
						<div>
							<dt class="text-xs font-bold tracking-wide text-slate-400 uppercase">Publicado</dt>
							<dd class="mt-1 font-bold text-slate-800">
								{formatDate(game.published_at) ?? 'Data não informada'}
							</dd>
						</div>
					</dl>
				</div>
			</article>

			<aside class="arcade-panel-blue p-6 lg:sticky lg:top-24">
				<p class="eyebrow">Multiplayer</p>
				<h2 class="display-title mt-3 text-2xl text-slate-950">Criar uma sala</h2>
				<p class="mt-2 text-sm leading-6 text-slate-600">
					Escolha como os outros jogadores verão você e compartilhe o convite após criar a sala.
				</p>

				<form
					onsubmit={(event) => {
						event.preventDefault();
						void createRoom();
					}}
					class="mt-6 space-y-4"
				>
					<label class="block">
						<span class="mb-1.5 block text-sm font-bold text-slate-700">Seu nickname</span>
						<input
							type="text"
							bind:value={nickname}
							oninput={() => (nicknameWasEdited = true)}
							required
							maxlength="30"
							autocomplete="nickname"
							placeholder="Como você aparecerá na sala"
							class="form-control"
						/>
					</label>

					<button
						type="submit"
						disabled={isCreating}
						class="primary-button w-full disabled:cursor-not-allowed disabled:opacity-50"
					>
						{isCreating ? 'Criando sala...' : 'Criar sala'}
					</button>
				</form>

				{#if createError}
					<div class="mt-4"><Alert tone="error" message={createError} /></div>
				{/if}
			</aside>
		</div>
	{/if}
</main>
