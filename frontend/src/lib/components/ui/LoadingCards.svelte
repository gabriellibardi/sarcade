<script lang="ts">
	let { count = 3 }: { count?: number } = $props();
	let placeholders = $derived(Array.from({ length: count }, (_, index) => index));
</script>

<div class="grid gap-5 md:grid-cols-2 lg:grid-cols-3" aria-label="Carregando conteúdo">
	{#each placeholders as placeholder (placeholder)}
		<div class="h-96 animate-pulse rounded-3xl border border-slate-200 bg-white shadow-sm"></div>
	{/each}
</div>
