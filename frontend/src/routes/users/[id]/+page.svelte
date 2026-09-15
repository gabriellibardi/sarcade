<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import GameCard from '$lib/components/GameCard.svelte';
	import type { PublicUserProfile } from '$lib/types/game';

	let profile: PublicUserProfile | null = null;
	let isLoading = true;
	let notFound = false;
	let error = '';

	const userId = Number(page.params.id);

	async function loadProfile() {
		isLoading = true;
		notFound = false;
		error = '';

		if (!Number.isSafeInteger(userId) || userId <= 0) {
			notFound = true;
			isLoading = false;
			return;
		}

		try {
			const response = await fetch(`/api/users/${userId}`);

			if (response.status === 404) {
				notFound = true;
				return;
			}

			const result = await response.json().catch(() => null);

			if (!response.ok || !result) {
				error = 'Não foi possível carregar este perfil.';
				return;
			}

			profile = result;
		} catch (loadError) {
			error = 'Falha ao conectar com o servidor.';
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	function formatDate(value: string) {
		const normalized = value.includes('T') ? value : `${value.replace(' ', 'T')}Z`;
		const date = new Date(normalized);

		return Number.isNaN(date.getTime())
			? 'Data não informada'
			: new Intl.DateTimeFormat('pt-BR', { month: 'long', year: 'numeric' }).format(date);
	}

	onMount(loadProfile);
</script>

<svelte:head>
	<title>{profile ? `${profile.name} — Sarcade` : 'Perfil — Sarcade'}</title>
	<meta name="description" content="Veja os jogos publicados por um autor no Sarcade." />
</svelte:head>

<main class="mx-auto max-w-6xl px-5 py-12 sm:px-6 lg:py-16">
	<a href={resolve('/catalog')} class="text-sm font-bold text-indigo-700 hover:underline"
		>← Voltar ao catálogo</a
	>

	{#if isLoading}
		<div class="mt-7 h-56 animate-pulse rounded-3xl bg-white" aria-label="Carregando perfil"></div>
	{:else if notFound}
		<section
			class="mt-7 rounded-3xl border border-dashed border-slate-300 bg-white p-12 text-center"
		>
			<h1 class="text-3xl font-black text-slate-950">Usuário não encontrado</h1>
			<p class="mt-3 text-slate-600">Não encontramos um perfil com esse identificador.</p>
			<a href={resolve('/catalog')} class="primary-button mt-6">Explorar o catálogo</a>
		</section>
	{:else if error}
		<section class="mt-7 rounded-2xl border border-red-200 bg-red-50 p-6 text-red-800">
			<p>{error}</p>
			<button type="button" on:click={loadProfile} class="mt-3 font-bold underline"
				>Tentar novamente</button
			>
		</section>
	{:else if profile}
		<header
			class="mt-7 flex flex-col gap-5 rounded-3xl border border-slate-200 bg-white p-7 shadow-sm sm:flex-row sm:items-center"
		>
			{#if profile.avatar}
				<img
					src={profile.avatar}
					alt=""
					referrerpolicy="no-referrer"
					class="h-24 w-24 rounded-3xl border border-slate-200 object-cover"
				/>
			{:else}
				<span
					class="flex h-24 w-24 items-center justify-center rounded-3xl bg-indigo-100 text-4xl font-black text-indigo-700"
				>
					{profile.name.slice(0, 1).toUpperCase()}
				</span>
			{/if}

			<div>
				<p class="text-sm font-bold tracking-widest text-indigo-600 uppercase">Autor no Sarcade</p>
				<h1 class="mt-1 text-4xl font-black tracking-tight text-slate-950">{profile.name}</h1>
				<p class="mt-2 text-slate-600">
					Membro desde {formatDate(profile.created_at)} · {profile.published_games.length === 1
						? '1 jogo publicado'
						: `${profile.published_games.length} jogos publicados`}
				</p>
			</div>
		</header>

		<section class="mt-12">
			<h2 class="text-3xl font-black tracking-tight text-slate-950">Jogos publicados</h2>

			{#if profile.published_games.length === 0}
				<div
					class="mt-6 rounded-2xl border border-dashed border-slate-300 bg-white/70 p-10 text-center"
				>
					<p class="font-bold text-slate-800">Este autor ainda não possui jogos publicados.</p>
				</div>
			{:else}
				<div class="mt-6 grid gap-5 md:grid-cols-2 lg:grid-cols-3">
					{#each profile.published_games as game (game.id)}
						<GameCard {game} />
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</main>
