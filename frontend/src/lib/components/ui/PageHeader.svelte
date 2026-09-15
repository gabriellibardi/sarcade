<script lang="ts">
	let {
		eyebrow,
		title,
		description,
		centered = false
	}: { eyebrow?: string; title: string; description?: string; centered?: boolean } = $props();
</script>

<header class={centered ? 'mx-auto max-w-3xl text-center' : 'max-w-3xl'}>
	{#if eyebrow}
		<p class="eyebrow">{eyebrow}</p>
	{/if}
	<h1 class="mt-2 text-4xl font-black tracking-tight text-slate-950 sm:text-5xl">{title}</h1>
	{#if description}
		<p class="mt-4 text-lg leading-8 text-slate-600">{description}</p>
	{/if}
</header>
