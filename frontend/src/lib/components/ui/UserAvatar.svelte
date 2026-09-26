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
		class={`shrink-0 border-2 border-slate-300 object-cover shadow-[3px_3px_0_#3d1a62] ${sizeClasses[size]}`}
	/>
{:else}
	<span
		class={`flex shrink-0 items-center justify-center border-2 border-indigo-300 bg-indigo-100 font-black text-indigo-300 shadow-[3px_3px_0_#3d1a62] ${sizeClasses[size]}`}
		aria-hidden="true"
	>
		{name.slice(0, 1).toUpperCase()}
	</span>
{/if}
