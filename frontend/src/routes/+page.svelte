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

<main class="min-h-[calc(100svh-5rem)]">
	<section>
		<div class="mx-auto max-w-5xl px-5 pt-16 pb-12 sm:px-6 lg:pt-20 lg:pb-14">
			<h1 class="flex justify-center"><BrandWordmark size="hero" /></h1>
			<p class="mx-auto mt-7 max-w-xl text-center text-sm leading-6 text-slate-700 sm:text-base">
				Descubra novos jogos, reúna seus amigos e comece uma partida.
			</p>
		</div>
	</section>

	<section>
		<div class="mx-auto max-w-2xl px-5 pb-24 sm:px-6">
			<div class="mb-5 flex items-center justify-between gap-4">
				<h2 class="text-xs font-medium tracking-[0.08em] text-slate-800 uppercase">Destaques</h2>
				<a href={resolve('/catalog')} class="secondary-button px-5 py-2 text-[11px]">Ver catálogo</a
				>
			</div>

			{#if isLoading}
				<LoadingCards variant="compact" />
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
				<div class="grid gap-3 sm:grid-cols-3">
					{#each previewGames as game (game.id)}
						<GameCard {game} variant="compact" />
					{/each}
				</div>
			{/if}
		</div>
	</section>
</main>
