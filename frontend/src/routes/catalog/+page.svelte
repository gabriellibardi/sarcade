<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { apiRequest, errorMessage } from '$lib/api';
	import GameCard from '$lib/components/GameCard.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingCards from '$lib/components/ui/LoadingCards.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
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
			games = await apiRequest<GameSummary[]>('/api/games');
		} catch (loadError) {
			error = errorMessage(loadError, 'Não foi possível carregar o catálogo.');
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	function clearFilters() {
		search = '';
		runtime = 'all';
	}

	onMount(loadGames);
</script>

<svelte:head>
	<title>Catálogo de jogos — Sarcade</title>
	<meta name="description" content="Explore os jogos aprovados disponíveis no Sarcade." />
</svelte:head>

<main class="page-shell">
	<PageHeader
		eyebrow="Jogos aprovados"
		title="Catálogo"
		description="Explore projetos revisados pela equipe e escolha o próximo jogo da sua sala."
	/>

	<section
		class="surface-card mt-9 flex flex-col gap-3 p-4 sm:flex-row"
		aria-label="Filtros do catálogo"
	>
		<label class="flex-1">
			<span class="sr-only">Pesquisar jogos</span>
			<input
				type="search"
				bind:value={search}
				placeholder="Pesquisar por título, descrição ou autor"
				class="form-control"
			/>
		</label>

		<label class="sm:w-56">
			<span class="sr-only">Filtrar por linguagem</span>
			<select bind:value={runtime} class="form-control">
				<option value="all">Todas as linguagens</option>
				{#each runtimes as [runtimeId, runtimeName] (runtimeId)}
					<option value={runtimeId}>{runtimeName}</option>
				{/each}
			</select>
		</label>
	</section>

	<div class="mt-5 flex items-center justify-between text-sm text-slate-500">
		<p>
			{filteredGames.length}
			{filteredGames.length === 1 ? 'jogo encontrado' : 'jogos encontrados'}
		</p>
		{#if search || runtime !== 'all'}
			<button
				type="button"
				onclick={clearFilters}
				class="font-mono text-xs font-bold text-indigo-300 uppercase hover:underline"
			>
				Limpar filtros
			</button>
		{/if}
	</div>

	<div class="mt-6">
		{#if isLoading}
			<LoadingCards count={6} />
		{:else if error}
			<Alert tone="error" message={error} onRetry={loadGames} />
		{:else if games.length === 0}
			<EmptyState
				title="Nenhum jogo aprovado ainda"
				description="Os jogos aparecerão aqui depois que uma revisão for aprovada."
			>
				<a href={resolve('/submit')} class="primary-button">Publicar jogo</a>
				<a href={resolve('/pending')} class="secondary-button">Ver revisões</a>
			</EmptyState>
		{:else if filteredGames.length === 0}
			<EmptyState
				title="Nenhum jogo corresponde aos filtros"
				description="Tente remover um filtro ou usar outro termo de busca."
			>
				<button type="button" onclick={clearFilters} class="secondary-button"
					>Mostrar catálogo completo</button
				>
			</EmptyState>
		{:else}
			<div class="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
				{#each filteredGames as game (game.id)}
					<GameCard {game} />
				{/each}
			</div>
		{/if}
	</div>
</main>
