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
	<a href={resolve('/catalog')} class="text-sm font-bold text-indigo-700 hover:underline"
		>← Voltar ao catálogo</a
	>

	{#if isLoading}
		<div
			class="mt-7 grid animate-pulse gap-8 lg:grid-cols-[1.3fr_0.7fr]"
			aria-label="Carregando jogo"
		>
			<div class="h-[34rem] rounded-3xl bg-white"></div>
			<div class="h-80 rounded-3xl bg-white"></div>
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
			<article class="overflow-hidden rounded-3xl border border-slate-200 bg-white shadow-sm">
				<div class="aspect-[16/9] bg-gradient-to-br from-indigo-100 via-sky-50 to-amber-100">
					{#if game.has_thumbnail}
						<img
							src={`/api/games/${game.id}/thumbnail`}
							alt={`Capa do jogo ${game.title}`}
							class="h-full w-full object-cover"
						/>
					{:else}
						<div
							class="flex h-full items-center justify-center text-7xl font-black text-indigo-300"
						>
							{game.title.slice(0, 1).toUpperCase()}
						</div>
					{/if}
				</div>

				<div class="p-6 sm:p-8">
					<div class="flex flex-wrap items-center gap-2">
						<span class="rounded-full bg-indigo-100 px-3 py-1 text-xs font-bold text-indigo-800">
							{game.runtime_name} · {game.runtime_version}
						</span>
						{#if game.is_featured}
							<span class="rounded-full bg-amber-300 px-3 py-1 text-xs font-bold text-amber-950"
								>Destaque</span
							>
						{/if}
					</div>

					<h1 class="mt-4 text-4xl font-black tracking-tight text-slate-950">{game.title}</h1>
					<p class="mt-5 text-lg leading-8 whitespace-pre-line text-slate-600">
						{game.description}
					</p>

					<dl class="mt-8 grid gap-4 border-t border-slate-100 pt-6 sm:grid-cols-3">
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
										class="text-indigo-700 hover:underline">{game.author_name}</a
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

			<aside class="rounded-3xl border border-indigo-200 bg-white p-6 shadow-sm lg:sticky lg:top-6">
				<p class="text-sm font-bold tracking-widest text-indigo-600 uppercase">Multiplayer</p>
				<h2 class="mt-2 text-2xl font-black text-slate-950">Criar uma sala</h2>
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
							class="block w-full rounded-xl border-slate-300 focus:border-indigo-500 focus:ring-indigo-500"
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
