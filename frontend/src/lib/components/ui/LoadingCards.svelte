<script lang="ts">
	let { count = 3, variant = 'default' }: { count?: number; variant?: 'default' | 'compact' } =
		$props();
	let placeholders = $derived(Array.from({ length: count }, (_, index) => index));
	let isCompact = $derived(variant === 'compact');
</script>

<div
	class={isCompact ? 'grid gap-3 sm:grid-cols-3' : 'grid gap-5 md:grid-cols-2 lg:grid-cols-3'}
	aria-label="Carregando conteúdo"
>
	{#each placeholders as placeholder (placeholder)}
		<div
			class={`${isCompact ? 'h-48 rounded-[3px]' : 'h-96 rounded-xl'} animate-pulse border border-white/10 bg-arcade-panel/85 shadow-2xl`}
		></div>
	{/each}
</div>
