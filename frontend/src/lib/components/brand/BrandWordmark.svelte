<script lang="ts">
	import horizontalLogo from '$lib/assets/logo-horizontal.svg';

	let { size = 'header' }: { size?: 'header' | 'hero' } = $props();

	const sizeClasses = {
		header: 'h-8 w-auto',
		hero: 'h-auto w-full max-w-3xl'
	};
</script>

<img src={horizontalLogo} alt="Sarcade" class={`block ${sizeClasses[size]}`} />
