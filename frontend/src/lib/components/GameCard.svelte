<script lang="ts">
	import { resolve } from '$app/paths';
	import type { GameSummary } from '$lib/types/game';

	let { game }: { game: GameSummary } = $props();

	const playerLabel = $derived(
		game.min_players === game.max_players
			? `${game.min_players} ${game.min_players === 1 ? 'jogador' : 'jogadores'}`
			: `${game.min_players}–${game.max_players} jogadores`
	);
</script>

<article
	class="group flex h-full flex-col overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm transition hover:-translate-y-1 hover:border-indigo-200 hover:shadow-lg"
>
	<div
		class="relative aspect-[16/9] overflow-hidden bg-gradient-to-br from-indigo-100 via-sky-50 to-amber-100"
	>
		{#if game.has_thumbnail}
			<img
				src={`/api/games/${game.id}/thumbnail`}
				alt={`Capa do jogo ${game.title}`}
				class="h-full w-full object-cover transition duration-300 group-hover:scale-[1.03]"
			/>
		{:else}
			<div class="flex h-full items-center justify-center" aria-label="Jogo sem imagem de capa">
				<span
					class="rounded-2xl bg-white/80 px-5 py-3 text-3xl font-black text-indigo-300 shadow-sm"
				>
					{game.title.slice(0, 1).toUpperCase()}
				</span>
			</div>
		{/if}

		<div class="absolute inset-x-3 top-3 flex items-start justify-between gap-2">
			<span
				class="rounded-full bg-slate-950/80 px-2.5 py-1 text-xs font-semibold text-white backdrop-blur"
			>
				{game.runtime_name}
			</span>
			{#if game.is_featured}
				<span class="rounded-full bg-amber-300 px-2.5 py-1 text-xs font-bold text-amber-950">
					Destaque
				</span>
			{/if}
		</div>
	</div>

	<div class="flex flex-1 flex-col p-5">
		<div class="mb-2 flex items-start justify-between gap-3">
			<h3 class="text-xl font-bold tracking-tight text-slate-950">
				<a
					href={resolve('/games/[id]', { id: String(game.id) })}
					class="hover:text-indigo-700 hover:underline">{game.title}</a
				>
			</h3>
			<span class="shrink-0 rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-600">
				{playerLabel}
			</span>
		</div>

		<p class="line-clamp-3 flex-1 text-sm leading-6 text-slate-600">{game.description}</p>

		<div class="mt-5 flex items-center justify-between gap-3 border-t border-slate-100 pt-4">
			<div class="min-w-0 text-xs text-slate-500">
				{#if game.author_id && game.author_name}
					<p class="truncate">
						por <a
							href={resolve('/users/[id]', { id: String(game.author_id) })}
							class="font-semibold hover:text-indigo-700 hover:underline">{game.author_name}</a
						>
					</p>
				{:else}
					<p class="truncate">por autor não informado</p>
				{/if}
				<p class="truncate">{game.runtime_version}</p>
			</div>

			<a
				href={resolve('/games/[id]', { id: String(game.id) })}
				class="primary-button shrink-0 px-4 py-2 text-sm">Ver jogo</a
			>
		</div>
	</div>
</article>
