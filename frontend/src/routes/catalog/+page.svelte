<script lang="ts">
	import { onMount } from 'svelte';
	import GameCard from '$lib/components/GameCard.svelte';
	import type { GameSummary } from '$lib/types/game';

	let games: GameSummary[] = [];
	let search = '';
	let runtime = 'all';
	let isLoading = true;
	let error = '';

	$: runtimes = Array.from(
		new Map(games.map((game) => [game.runtime, game.runtime_name])).entries()
	);
	$: normalizedSearch = search.trim().toLocaleLowerCase('pt-BR');
	$: filteredGames = games.filter((game) => {
		const matchesRuntime = runtime === 'all' || game.runtime === runtime;
		const matchesSearch =
			normalizedSearch.length === 0 ||
			game.title.toLocaleLowerCase('pt-BR').includes(normalizedSearch) ||
			game.description.toLocaleLowerCase('pt-BR').includes(normalizedSearch) ||
			(game.author_name ?? '').toLocaleLowerCase('pt-BR').includes(normalizedSearch);

		return matchesRuntime && matchesSearch;
	});

	async function loadGames() {
		isLoading = true;
		error = '';

		try {
			const response = await fetch('/api/games');
			const result = await response.json().catch(() => null);

			if (!response.ok || !Array.isArray(result)) {
				error = 'Não foi possível carregar o catálogo.';
				return;
			}

			games = result;
		} catch (loadError) {
			error = 'Falha ao conectar com o servidor.';
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	onMount(loadGames);
</script>

<svelte:head>
	<title>Catálogo de jogos — Sarcade</title>
	<meta name="description" content="Explore os jogos aprovados disponíveis no Sarcade." />
</svelte:head>

<main class="mx-auto max-w-6xl px-5 py-12 sm:px-6 lg:py-16">
	<header class="max-w-3xl">
		<p class="text-sm font-bold tracking-widest text-indigo-600 uppercase">Jogos aprovados</p>
		<h1 class="mt-2 text-4xl font-black tracking-tight text-slate-950 sm:text-5xl">Catálogo</h1>
		<p class="mt-4 text-lg leading-8 text-slate-600">
			Explore projetos revisados pela equipe e escolha o próximo jogo da sua sala.
		</p>
	</header>

	<div
		class="mt-9 flex flex-col gap-3 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm sm:flex-row"
	>
		<label class="flex-1">
			<span class="sr-only">Pesquisar jogos</span>
			<input
				type="search"
				bind:value={search}
				placeholder="Pesquisar por título, descrição ou autor"
				class="w-full rounded-xl border-slate-300 text-sm focus:border-indigo-500 focus:ring-indigo-500"
			/>
		</label>

		<label class="sm:w-56">
			<span class="sr-only">Filtrar por linguagem</span>
			<select
				bind:value={runtime}
				class="w-full rounded-xl border-slate-300 text-sm focus:border-indigo-500 focus:ring-indigo-500"
			>
				<option value="all">Todas as linguagens</option>
				{#each runtimes as [runtimeId, runtimeName]}
					<option value={runtimeId}>{runtimeName}</option>
				{/each}
			</select>
		</label>
	</div>

	<div class="mt-5 flex items-center justify-between text-sm text-slate-500">
		<p>
			{filteredGames.length}
			{filteredGames.length === 1 ? 'jogo encontrado' : 'jogos encontrados'}
		</p>
		{#if search || runtime !== 'all'}
			<button
				type="button"
				on:click={() => {
					search = '';
					runtime = 'all';
				}}
				class="font-semibold text-indigo-700 hover:underline"
			>
				Limpar filtros
			</button>
		{/if}
	</div>

	{#if isLoading}
		<div class="mt-6 grid gap-5 md:grid-cols-2 lg:grid-cols-3" aria-label="Carregando catálogo">
			{#each Array(6) as _}
				<div class="h-96 animate-pulse rounded-2xl border border-slate-200 bg-white"></div>
			{/each}
		</div>
	{:else if error}
		<div class="mt-6 rounded-2xl border border-red-200 bg-red-50 p-6 text-red-800">
			<p>{error}</p>
			<button type="button" on:click={loadGames} class="mt-3 font-bold underline"
				>Tentar novamente</button
			>
		</div>
	{:else if games.length === 0}
		<div
			class="mt-6 rounded-2xl border border-dashed border-slate-300 bg-white/70 p-12 text-center"
		>
			<p class="text-xl font-bold text-slate-800">Nenhum jogo aprovado ainda.</p>
			<p class="mt-2 text-slate-600">
				Os jogos aparecerão aqui depois que uma revisão for aprovada.
			</p>
			<div class="mt-6 flex flex-wrap justify-center gap-3">
				<a href="/submit" class="primary-button">Publicar jogo</a>
				<a href="/pending" class="secondary-button">Ver revisões</a>
			</div>
		</div>
	{:else if filteredGames.length === 0}
		<div
			class="mt-6 rounded-2xl border border-dashed border-slate-300 bg-white/70 p-12 text-center"
		>
			<p class="text-xl font-bold text-slate-800">Nenhum jogo corresponde aos filtros.</p>
			<button
				type="button"
				on:click={() => {
					search = '';
					runtime = 'all';
				}}
				class="mt-4 font-bold text-indigo-700 hover:underline"
			>
				Mostrar catálogo completo
			</button>
		</div>
	{:else}
		<div class="mt-6 grid gap-5 md:grid-cols-2 lg:grid-cols-3">
			{#each filteredGames as game (game.id)}
				<GameCard {game} />
			{/each}
		</div>
	{/if}
</main>
