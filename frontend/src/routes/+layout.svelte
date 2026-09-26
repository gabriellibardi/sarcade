<script lang="ts">
	import { onMount } from 'svelte';
	import SiteFooter from '$lib/components/layout/SiteFooter.svelte';
	import SiteHeader from '$lib/components/layout/SiteHeader.svelte';
	import { loadAuth } from '$lib/auth';
	import logotipo from '$lib/assets/logotipo.svg';
	import './layout.css';

	let { children } = $props();

	onMount(loadAuth);
</script>

<svelte:head>
	<link rel="icon" href={logotipo} />
	<meta
		name="description"
		content="Sarcade: jogos educacionais criados com linguagens para estudantes."
	/>
</svelte:head>

<div class="flex min-h-screen flex-col">
	<SiteHeader />
	<div class="flex-1">{@render children()}</div>
	<SiteFooter />
</div>
