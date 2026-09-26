<script lang="ts">
	import logotipo from '$lib/assets/logotipo.svg';

	let { size = 'medium' }: { size?: 'small' | 'medium' | 'large' } = $props();

	const sizeClasses = {
		small: 'h-9',
		medium: 'h-12',
		large: 'h-20 sm:h-24'
	};
</script>

<img src={logotipo} alt="" class={`block w-auto ${sizeClasses[size]}`} />
