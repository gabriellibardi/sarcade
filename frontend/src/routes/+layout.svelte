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
		content="Sarcade: crie, compartilhe e jogue jogos educacionais no navegador."
	/>
</svelte:head>

<div class="site-background" aria-hidden="true"></div>

<div class="relative z-10 flex min-h-screen flex-col">
	<SiteHeader />
	<div class="flex-1">{@render children()}</div>
	<SiteFooter />
</div>
