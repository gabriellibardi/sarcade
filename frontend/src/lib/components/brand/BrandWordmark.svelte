<script lang="ts">
	import horizontalLogo from '$lib/assets/logo-horizontal.svg';

	let { size = 'header' }: { size?: 'header' | 'hero' | 'footer' } = $props();

	const sizeClasses = {
		header: 'h-8 w-auto',
		hero: 'h-auto w-full max-w-3xl',
		footer: 'h-auto w-44 sm:w-52'
	};
</script>

<img src={horizontalLogo} alt="Sarcade" class={`block ${sizeClasses[size]}`} />
