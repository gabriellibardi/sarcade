<script lang="ts">
	let {
		name,
		avatar,
		size = 'medium'
	}: { name: string; avatar: string | null; size?: 'small' | 'medium' | 'large' } = $props();

	const sizeClasses = {
		small: 'h-9 w-9 text-sm',
		medium: 'h-11 w-11 text-base',
		large: 'h-24 w-24 text-4xl'
	};
</script>

{#if avatar}
	<img
		src={avatar}
		alt=""
		referrerpolicy="no-referrer"
		class={`shrink-0 rounded-full border border-white/15 object-cover shadow-lg ${sizeClasses[size]}`}
	/>
{:else}
	<span
		class={`flex shrink-0 items-center justify-center rounded-full border border-indigo-300/50 bg-indigo-100 font-bold text-indigo-300 shadow-lg ${sizeClasses[size]}`}
		aria-hidden="true"
	>
		{name.slice(0, 1).toUpperCase()}
	</span>
{/if}
