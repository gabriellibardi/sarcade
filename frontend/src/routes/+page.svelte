<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { apiRequest, errorMessage } from '$lib/api';
	import BrandWordmark from '$lib/components/brand/BrandWordmark.svelte';
	import GameCard from '$lib/components/GameCard.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingCards from '$lib/components/ui/LoadingCards.svelte';
	import type { GameSummary } from '$lib/types/game';

	let games: GameSummary[] = [];
	let isLoading = true;
	let error = '';

	$: previewGames = (
		games.some((game) => game.is_featured) ? games.filter((game) => game.is_featured) : games
	).slice(0, 3);

	async function loadGames() {
		isLoading = true;
		error = '';

		try {
			games = await apiRequest<GameSummary[]>('/api/games');
		} catch (loadError) {
			error = errorMessage(loadError, 'Não foi possível carregar o catálogo agora.');
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	onMount(loadGames);
</script>

<svelte:head>
	<title>Sarcade</title>
</svelte:head>

<main>
	<section class="relative overflow-hidden border-b-2 border-arcade-line">
		<div class="mx-auto max-w-6xl px-5 py-16 sm:px-6 lg:py-24">
			<h1 class="flex justify-center"><BrandWordmark size="hero" /></h1>

			<div class="mt-10 flex flex-wrap justify-center gap-4">
				<a href={resolve('/catalog')} class="primary-button">Escolher jogo</a>
				<a href={resolve('/submit')} class="secondary-button">Publicar meu jogo</a>
			</div>
		</div>
	</section>

	<section class="bg-[#101010]/70">
		<div class="mx-auto max-w-6xl px-5 py-16 sm:px-6 lg:py-20">
			<div class="mb-8 flex flex-wrap items-end justify-between gap-4">
				<div>
					<p class="eyebrow">Now playing</p>
					<h2 class="display-title mt-3 text-4xl text-slate-950">Jogos em destaque</h2>
					<p class="mt-3 text-slate-600">Projetos aprovados e prontos para receber jogadores.</p>
				</div>
				<a href={resolve('/catalog')} class="secondary-button py-2.5 text-sm">Ver catálogo</a>
			</div>

			{#if isLoading}
				<LoadingCards />
			{:else if error}
				<Alert tone="error" message={error} onRetry={loadGames} />
			{:else if previewGames.length === 0}
				<EmptyState
					title="O catálogo ainda está vazio"
					description="Publique um jogo ou aprove uma revisão para ele aparecer aqui."
				>
					<a href={resolve('/submit')} class="primary-button">Publicar jogo</a>
					<a href={resolve('/pending')} class="secondary-button">Ver revisões</a>
				</EmptyState>
			{:else}
				<div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
					{#each previewGames as game (game.id)}
						<GameCard {game} />
					{/each}
				</div>
			{/if}
		</div>
	</section>
</main>
