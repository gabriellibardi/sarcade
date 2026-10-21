<script lang="ts">
	import { resolve } from '$app/paths';
	import type { GameSummary } from '$lib/types/game';
	import { playerCountLabel } from '$lib/utils/format';

	let { game }: { game: GameSummary } = $props();

	const playerLabel = $derived(playerCountLabel(game.min_players, game.max_players));
</script>

<article
	class="group relative flex h-full flex-col overflow-hidden border-2 border-arcade-line bg-arcade-panel shadow-[6px_6px_0_#511243] transition duration-150 focus-within:-translate-y-1 focus-within:border-arcade-red focus-within:shadow-[8px_8px_0_#511243] hover:-translate-y-1 hover:border-arcade-red hover:shadow-[8px_8px_0_#511243]"
>
	<a
		href={resolve('/games/[id]', { id: String(game.id) })}
		class="absolute inset-0 z-10"
		aria-label={`Abrir detalhes de ${game.title}`}
	></a>

	<div class="relative aspect-[16/9] overflow-hidden bg-[#181719]">
		{#if game.has_thumbnail}
			<img
				src={`/api/games/${game.id}/thumbnail`}
				alt={`Capa do jogo ${game.title}`}
				class="h-full w-full object-cover opacity-90 transition duration-300 group-hover:scale-[1.03] group-hover:opacity-100"
			/>
		{:else}
			<div
				class="flex h-full items-center justify-center bg-[linear-gradient(135deg,#25181c_0_50%,#191c2d_50%)]"
				aria-label="Jogo sem imagem de capa"
			>
				<span
					class="display-title border-2 border-slate-300 bg-arcade-bg px-6 py-3 text-5xl text-arcade-ink shadow-[5px_5px_0_#3d1a62]"
				>
					{game.title.slice(0, 1).toUpperCase()}
				</span>
			</div>
		{/if}

		<div
			class="absolute inset-x-3 top-3 flex items-start justify-between gap-2 font-mono uppercase"
		>
			<span
				class="border border-indigo-300 bg-arcade-bg/90 px-2.5 py-1 text-[10px] font-bold text-indigo-300"
			>
				{game.runtime_name}
			</span>
			{#if game.is_featured}
				<span
					class="border border-arcade-gold bg-arcade-gold px-2.5 py-1 text-[10px] font-black text-[#21160a]"
				>
					Destaque
				</span>
			{/if}
		</div>
	</div>

	<div class="flex flex-1 flex-col p-5">
		<div class="mb-3 flex items-start justify-between gap-3">
			<h3 class="text-xl font-black tracking-tight text-slate-950 group-hover:text-indigo-300">
				{game.title}
			</h3>
			<span
				class="status-chip shrink-0 bg-slate-100 px-2 py-1 text-[10px] font-bold text-slate-600"
			>
				{playerLabel}
			</span>
		</div>

		<p class="line-clamp-3 flex-1 text-sm leading-6 text-slate-600">{game.description}</p>

		<div class="mt-5 flex items-center justify-between gap-3 border-t-2 border-arcade-line pt-4">
			<div class="min-w-0 font-mono text-[10px] leading-5 text-slate-500 uppercase">
				{#if game.author_id && game.author_name}
					<p class="truncate">
						por <a
							href={resolve('/users/[id]', { id: String(game.author_id) })}
							class="relative z-20 font-bold text-slate-700 hover:text-indigo-300 hover:underline"
							>{game.author_name}</a
						>
					</p>
				{:else}
					<p class="truncate">autor não informado</p>
				{/if}
				<p class="truncate">{game.runtime_version}</p>
			</div>

			<span class="primary-button shrink-0 px-4 py-2 text-xs" aria-hidden="true">Jogar</span>
		</div>
	</div>
</article>
