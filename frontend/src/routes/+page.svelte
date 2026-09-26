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
	<title>Sarcade — jogos feitos por estudantes</title>
</svelte:head>

<main>
	<section class="relative overflow-hidden border-b-2 border-arcade-line">
		<div class="pointer-events-none absolute inset-0 opacity-35" aria-hidden="true">
			<div class="absolute top-8 -left-12 h-24 w-80 rotate-[-8deg] bg-arcade-red"></div>
			<div
				class="absolute right-[-5rem] bottom-14 h-28 w-[30rem] rotate-[-8deg] bg-arcade-blue"
			></div>
		</div>

		<div class="relative mx-auto max-w-6xl px-5 py-16 sm:px-6 lg:py-24">
			<div class="mb-12 flex items-center justify-between gap-4 border-b-2 border-arcade-line pb-4">
				<p class="arcade-kicker">Insert coin · Press start</p>
				<p class="hidden font-mono text-[10px] tracking-[0.18em] text-slate-500 uppercase sm:block">
					Jogos criados por estudantes
				</p>
			</div>

			<h1 class="flex justify-center"><BrandWordmark size="hero" /></h1>

			<div class="mx-auto mt-14 max-w-3xl text-center">
				<h2 class="display-title text-4xl text-slate-950 sm:text-6xl">
					Jogos de estudantes.<br /><span class="text-arcade-gold">Diversão de arcade.</span>
				</h2>
				<p class="mt-6 text-lg leading-8 text-slate-600">
					Descubra jogos criados com linguagens educacionais, reúna seus amigos em uma sala e
					coloque também a sua criação no fliperama.
				</p>

				<div class="mt-8 flex flex-wrap justify-center gap-4">
					<a href={resolve('/catalog')} class="primary-button">Escolher jogo</a>
					<a href={resolve('/submit')} class="secondary-button">Publicar meu jogo</a>
				</div>
			</div>
		</div>
	</section>

	<section class="mx-auto max-w-6xl px-5 py-16 sm:px-6">
		<div class="mb-7 flex items-end justify-between gap-4">
			<div>
				<p class="eyebrow">How to play</p>
				<h2 class="display-title mt-3 text-3xl text-slate-950 sm:text-4xl">
					Três passos. Uma partida.
				</h2>
			</div>
			<span class="hidden font-mono text-xs text-slate-500 sm:block">READY?</span>
		</div>

		<div class="grid gap-5 md:grid-cols-3">
			<a href={resolve('/catalog')} class="arcade-panel-red group p-6">
				<span
					class="font-mono text-4xl font-black text-arcade-red transition group-hover:text-[#d36a6a]"
					>01</span
				>
				<h3 class="mt-8 text-xl font-black text-slate-950">Encontre um jogo</h3>
				<p class="mt-2 text-sm leading-6 text-slate-600">
					Escolha entre projetos revisados e aprovados.
				</p>
			</a>
			<a href={resolve('/catalog')} class="arcade-panel-blue group p-6">
				<span
					class="font-mono text-4xl font-black text-arcade-blue transition group-hover:text-indigo-300"
					>02</span
				>
				<h3 class="mt-8 text-xl font-black text-slate-950">Abra uma sala</h3>
				<p class="mt-2 text-sm leading-6 text-slate-600">
					Defina seu apelido e chame a turma para jogar.
				</p>
			</a>
			<a href={resolve('/submit')} class="arcade-panel-red group p-6">
				<span class="font-mono text-4xl font-black text-arcade-gold">03</span>
				<h3 class="mt-8 text-xl font-black text-slate-950">Entre no catálogo</h3>
				<p class="mt-2 text-sm leading-6 text-slate-600">Envie seu código e acompanhe a revisão.</p>
			</a>
		</div>
	</section>

	<section class="border-t-2 border-arcade-line bg-[#101010]/70">
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
