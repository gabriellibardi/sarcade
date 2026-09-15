<script lang="ts">
	let {
		name,
		avatar,
		size = 'medium'
	}: { name: string; avatar: string | null; size?: 'small' | 'medium' | 'large' } = $props();

	const sizeClasses = {
		small: 'h-9 w-9 rounded-full text-sm',
		medium: 'h-11 w-11 rounded-2xl text-base',
		large: 'h-24 w-24 rounded-3xl text-4xl'
	};
</script>

{#if avatar}
	<img
		src={avatar}
		alt=""
		referrerpolicy="no-referrer"
		class={`shrink-0 border border-slate-200 object-cover ${sizeClasses[size]}`}
	/>
{:else}
	<span
		class={`flex shrink-0 items-center justify-center bg-indigo-100 font-black text-indigo-700 ${sizeClasses[size]}`}
		aria-hidden="true"
	>
		{name.slice(0, 1).toUpperCase()}
	</span>
{/if}
