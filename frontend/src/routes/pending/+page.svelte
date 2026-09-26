<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { apiRequest, errorMessage } from '$lib/api';
	import { authState, loadAuth } from '$lib/auth';
	import ReviewCard from '$lib/components/review/ReviewCard.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import type { GameSummary } from '$lib/types/game';

	let games: GameSummary[] = [];
	let isLoading = true;
	let error = '';

	async function loadPendingGames() {
		isLoading = true;
		error = '';

		try {
			games = await apiRequest<GameSummary[]>('/api/games/pending');
		} catch (loadError) {
			error = errorMessage(loadError, 'Não foi possível carregar as revisões pendentes.');
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	async function initializePage() {
		await loadAuth();

		if (!get(authState).user?.is_admin) {
			await goto(resolve('/'));
			return;
		}

		await loadPendingGames();
	}

	onMount(() => {
		void initializePage();
	});
</script>

<svelte:head>
	<title>Revisões pendentes — Sarcade</title>
	<meta name="description" content="Fila administrativa de jogos aguardando revisão." />
</svelte:head>

<main class="page-shell max-w-5xl">
	<PageHeader
		eyebrow="Área administrativa"
		title="Revisões pendentes"
		description="Analise o código, os metadados e a compatibilidade dos jogos antes de disponibilizá-los no catálogo."
	/>

	<div class="mt-8">
		{#if isLoading}
			<div class="space-y-4" aria-label="Carregando revisões">
				{#each [1, 2, 3] as placeholder (placeholder)}
					<div class="h-48 animate-pulse border-2 border-slate-200 bg-arcade-panel"></div>
				{/each}
			</div>
		{:else if error}
			<Alert tone="error" message={error} onRetry={loadPendingGames} />
		{:else if games.length === 0}
			<EmptyState
				title="Fila em dia"
				description="Não há nenhuma revisão aguardando análise neste momento."
			/>
		{:else}
			<div class="space-y-4">
				{#each games as game (game.revision_id)}
					<ReviewCard {game} />
				{/each}
			</div>
		{/if}
	</div>
</main>
