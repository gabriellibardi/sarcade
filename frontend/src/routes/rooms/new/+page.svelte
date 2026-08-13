<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import type { GameSummary } from '$lib/types/game';

	let games: GameSummary[] = [];
	let gameId = 0;
	let nickname = '';

	let isLoadingGames = true;
	let isCreating = false;
	let error = '';

	$: selectedGame = games.find((game) => game.id === gameId) ?? null;

	async function loadGames() {
		isLoadingGames = true;
		error = '';

		try {
			const response = await fetch('/api/games');
			const result = await response.json().catch(() => null);

			if (!response.ok || !Array.isArray(result)) {
				error = 'Não foi possível carregar os jogos disponíveis.';
				return;
			}

			games = result;

			const requestedId = Number(new URLSearchParams(window.location.search).get('game_id'));
			gameId = games.some((game) => game.id === requestedId) ? requestedId : (games[0]?.id ?? 0);
		} catch (loadError) {
			error = 'Falha ao conectar com o servidor.';
			console.error(loadError);
		} finally {
			isLoadingGames = false;
		}
	}

	async function createRoom() {
		isCreating = true;
		error = '';

		if (!selectedGame) {
			error = 'Selecione um jogo aprovado.';
			isCreating = false;
			return;
		}

		try {
			const response = await fetch('/api/rooms', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					game_id: gameId,
					nickname
				})
			});

			const result = await response.json().catch(() => null);

			if (!response.ok) {
				error = typeof result === 'string' ? result : (result?.message ?? 'Erro ao criar sala.');
				return;
			}

			await goto(
				`/rooms/${result.code}?nickname=${encodeURIComponent(nickname)}&host_token=${encodeURIComponent(result.host_token)}`
			);
		} catch (createError) {
			error = 'Falha ao conectar com o servidor.';
			console.error(createError);
		} finally {
			isCreating = false;
		}
	}

	onMount(loadGames);
</script>

<svelte:head>
	<title>Criar sala — Sarcade</title>
</svelte:head>

<main class="mx-auto grid max-w-5xl gap-8 px-5 py-12 sm:px-6 lg:grid-cols-[1fr_0.75fr] lg:py-16">
	<section>
		<p class="text-sm font-bold tracking-widest text-indigo-600 uppercase">Multiplayer</p>
		<h1 class="mt-2 text-4xl font-black tracking-tight text-slate-950">Criar uma sala</h1>
		<p class="mt-3 max-w-xl leading-7 text-slate-600">
			Escolha um jogo aprovado e informe como você quer aparecer para os outros jogadores.
		</p>

		<form
			on:submit|preventDefault={createRoom}
			class="mt-8 space-y-5 rounded-2xl border border-slate-200 bg-white p-6 shadow-sm"
		>
			<div>
				<label for="gameId" class="mb-1.5 block text-sm font-bold text-slate-700">Jogo</label>
				<select
					id="gameId"
					bind:value={gameId}
					disabled={isLoadingGames || games.length === 0}
					required
					class="block w-full rounded-xl border-slate-300 focus:border-indigo-500 focus:ring-indigo-500"
				>
					{#if isLoadingGames}
						<option value={0}>Carregando catálogo...</option>
					{:else if games.length === 0}
						<option value={0}>Nenhum jogo aprovado</option>
					{:else}
						{#each games as game}
							<option value={game.id}>{game.title} — {game.runtime_name}</option>
						{/each}
					{/if}
				</select>
			</div>

			<div>
				<label for="nickname" class="mb-1.5 block text-sm font-bold text-slate-700"
					>Seu apelido</label
				>
				<input
					id="nickname"
					type="text"
					bind:value={nickname}
					required
					maxlength="30"
					placeholder="Como os outros jogadores verão você"
					class="block w-full rounded-xl border-slate-300 focus:border-indigo-500 focus:ring-indigo-500"
				/>
			</div>

			<button
				type="submit"
				disabled={isCreating || isLoadingGames || !selectedGame}
				class="primary-button w-full disabled:cursor-not-allowed disabled:opacity-50"
			>
				{isCreating ? 'Criando sala...' : 'Criar sala'}
			</button>
		</form>

		{#if error}
			<div class="mt-4 rounded-xl border border-red-200 bg-red-50 p-4 text-sm text-red-800">
				<p>{error}</p>
				{#if games.length === 0 && !isLoadingGames}
					<a href="/catalog" class="mt-2 inline-block font-bold underline">Voltar ao catálogo</a>
				{/if}
			</div>
		{/if}
	</section>

	<aside class="lg:pt-16">
		{#if selectedGame}
			<div class="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm">
				<div class="aspect-[16/9] bg-gradient-to-br from-indigo-100 via-sky-50 to-amber-100">
					{#if selectedGame.has_thumbnail}
						<img
							src={`/api/games/${selectedGame.id}/thumbnail`}
							alt={`Capa do jogo ${selectedGame.title}`}
							class="h-full w-full object-cover"
						/>
					{:else}
						<div
							class="flex h-full items-center justify-center text-5xl font-black text-indigo-300"
						>
							{selectedGame.title.slice(0, 1).toUpperCase()}
						</div>
					{/if}
				</div>
				<div class="p-5">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<h2 class="text-xl font-black text-slate-950">{selectedGame.title}</h2>
						<span class="rounded-full bg-indigo-100 px-2.5 py-1 text-xs font-bold text-indigo-800">
							{selectedGame.runtime_name}
						</span>
					</div>
					<p class="mt-2 text-sm leading-6 text-slate-600">{selectedGame.description}</p>
					<p class="mt-4 text-sm font-semibold text-slate-700">
						{selectedGame.min_players === selectedGame.max_players
							? `${selectedGame.min_players} jogadores`
							: `${selectedGame.min_players}–${selectedGame.max_players} jogadores`}
					</p>
				</div>
			</div>
		{:else}
			<div
				class="rounded-2xl border border-dashed border-slate-300 bg-white/60 p-8 text-center text-slate-600"
			>
				{isLoadingGames ? 'Carregando jogo...' : 'Selecione um jogo para ver os detalhes.'}
			</div>
		{/if}
	</aside>
</main>
