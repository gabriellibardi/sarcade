<script lang="ts">
	import { onMount } from 'svelte';
	import GameCard from '$lib/components/GameCard.svelte';
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
			const response = await fetch('/api/games');
			const result = await response.json().catch(() => null);

			if (!response.ok || !Array.isArray(result)) {
				error = 'Não foi possível carregar o catálogo agora.';
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
	<title>Sarcade — jogos feitos por estudantes</title>
</svelte:head>

<main>
	<section class="overflow-hidden border-b border-indigo-100">
		<div
			class="mx-auto grid max-w-6xl items-center gap-12 px-5 py-16 sm:px-6 lg:grid-cols-[1.2fr_0.8fr] lg:py-24"
		>
			<div>
				<p class="mb-4 text-sm font-bold tracking-widest text-indigo-600 uppercase">
					Aprenda, crie e jogue
				</p>
				<h1 class="max-w-3xl text-4xl font-black tracking-tight text-slate-950 sm:text-6xl">
					Um arcade feito com linguagens para estudantes.
				</h1>
				<p class="mt-6 max-w-2xl text-lg leading-8 text-slate-600">
					Descubra jogos criados em Sgleam e Spython, reúna seus amigos em uma sala e compartilhe
					também os seus próprios projetos.
				</p>

				<div class="mt-8 flex flex-wrap gap-3">
					<a href="/catalog" class="primary-button">Explorar catálogo</a>
					<a href="/submit" class="secondary-button">Publicar meu jogo</a>
				</div>
			</div>

			<div class="relative mx-auto w-full max-w-md">
				<div class="absolute -inset-5 -z-10 rounded-[2rem] bg-indigo-200/50 blur-3xl"></div>
				<div class="rotate-2 rounded-3xl border border-indigo-200 bg-white p-6 shadow-xl">
					<div class="mb-6 flex items-center justify-between">
						<div>
							<p class="text-xs font-bold tracking-widest text-slate-400 uppercase">No Sarcade</p>
							<p class="mt-1 text-2xl font-black text-slate-950">Escolha sua linguagem</p>
						</div>
						<span
							class="flex h-12 w-12 items-center justify-center rounded-2xl bg-indigo-600 text-2xl text-white"
						>
							⌁
						</span>
					</div>

					<div class="grid gap-3 sm:grid-cols-2">
						<div class="rounded-2xl bg-indigo-50 p-4">
							<p class="font-black text-indigo-950">Sgleam</p>
							<p class="mt-1 text-sm text-indigo-700">Jogos com Gleam</p>
							<span
								class="mt-5 inline-block rounded-full bg-indigo-600 px-2.5 py-1 text-xs font-bold text-white"
							>
								.gleam
							</span>
						</div>
						<div class="rounded-2xl bg-amber-50 p-4">
							<p class="font-black text-amber-950">Spython</p>
							<p class="mt-1 text-sm text-amber-700">Jogos com Python</p>
							<span
								class="mt-5 inline-block rounded-full bg-amber-400 px-2.5 py-1 text-xs font-bold text-amber-950"
							>
								.py
							</span>
						</div>
					</div>
				</div>
			</div>
		</div>
	</section>

	<section class="mx-auto max-w-6xl px-5 py-14 sm:px-6">
		<div class="grid gap-4 md:grid-cols-3">
			<a
				href="/catalog"
				class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm transition hover:border-indigo-200 hover:shadow-md"
			>
				<span
					class="mb-4 flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-100 font-black text-indigo-700"
					>01</span
				>
				<h2 class="font-bold text-slate-950">Encontre um jogo</h2>
				<p class="mt-1 text-sm leading-6 text-slate-600">
					Veja apenas projetos já revisados e aprovados.
				</p>
			</a>
			<a
				href="/rooms/new"
				class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm transition hover:border-indigo-200 hover:shadow-md"
			>
				<span
					class="mb-4 flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-100 font-black text-emerald-700"
					>02</span
				>
				<h2 class="font-bold text-slate-950">Crie uma sala</h2>
				<p class="mt-1 text-sm leading-6 text-slate-600">
					Selecione o jogo, escolha seu apelido e convide os amigos.
				</p>
			</a>
			<a
				href="/submit"
				class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm transition hover:border-indigo-200 hover:shadow-md"
			>
				<span
					class="mb-4 flex h-10 w-10 items-center justify-center rounded-xl bg-amber-100 font-black text-amber-700"
					>03</span
				>
				<h2 class="font-bold text-slate-950">Compartilhe sua criação</h2>
				<p class="mt-1 text-sm leading-6 text-slate-600">
					Envie seu código e acompanhe o processo de revisão.
				</p>
			</a>
		</div>
	</section>

	<section class="mx-auto max-w-6xl px-5 pb-20 sm:px-6">
		<div class="mb-7 flex flex-wrap items-end justify-between gap-4">
			<div>
				<p class="text-sm font-bold text-indigo-600">CATÁLOGO</p>
				<h2 class="mt-1 text-3xl font-black tracking-tight text-slate-950">Jogos em destaque</h2>
				<p class="mt-2 text-slate-600">Projetos aprovados e prontos para receber jogadores.</p>
			</div>
			<a href="/catalog" class="secondary-button py-2.5 text-sm">Ver todos</a>
		</div>

		{#if isLoading}
			<div class="grid gap-5 md:grid-cols-3" aria-label="Carregando jogos">
				{#each Array(3) as _}
					<div class="h-96 animate-pulse rounded-2xl border border-slate-200 bg-white"></div>
				{/each}
			</div>
		{:else if error}
			<div class="rounded-2xl border border-red-200 bg-red-50 p-6 text-red-800">
				<p>{error}</p>
				<button type="button" on:click={loadGames} class="mt-3 font-bold underline"
					>Tentar novamente</button
				>
			</div>
		{:else if previewGames.length === 0}
			<div class="rounded-2xl border border-dashed border-slate-300 bg-white/70 p-10 text-center">
				<p class="text-lg font-bold text-slate-800">O catálogo ainda está vazio.</p>
				<p class="mt-2 text-slate-600">
					Publique um jogo ou aprove uma revisão para ele aparecer aqui.
				</p>
				<div class="mt-5 flex flex-wrap justify-center gap-3">
					<a href="/submit" class="primary-button">Publicar jogo</a>
					<a href="/pending" class="secondary-button">Ver revisões</a>
				</div>
			</div>
		{:else}
			<div class="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
				{#each previewGames as game (game.id)}
					<GameCard {game} />
				{/each}
			</div>
		{/if}
	</section>
</main>
