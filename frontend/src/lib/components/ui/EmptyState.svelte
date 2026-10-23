<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		description,
		children
	}: { title: string; description?: string; children?: Snippet } = $props();
</script>

<div
	class="rounded-2xl border border-dashed border-white/15 bg-arcade-panel/75 p-10 text-center shadow-2xl backdrop-blur-md sm:p-12"
>
	<div
		class="mx-auto flex h-12 w-12 items-center justify-center rounded-full border border-white/10 bg-slate-100 text-xl text-indigo-300"
		aria-hidden="true"
	>
		◇
	</div>
	<h2 class="display-title mt-4 text-2xl text-slate-900">{title}</h2>
	{#if description}
		<p class="mx-auto mt-2 max-w-xl leading-7 text-slate-600">{description}</p>
	{/if}
	{#if children}
		<div class="mt-6 flex flex-wrap justify-center gap-3">{@render children()}</div>
	{/if}
</div>
