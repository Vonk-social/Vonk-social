<script lang="ts">
	import type { Visibility } from '$lib/api/posts';

	type Props = { value: Visibility };
	let { value = $bindable('public') }: Props = $props();

	const options: { value: Visibility; label: string; help: string }[] = [
		{ value: 'public', label: 'Publiek', help: 'Iedereen kan dit zien' },
		{ value: 'followers', label: 'Volgers', help: 'Alleen wie jou volgt' },
		{ value: 'mentioned', label: 'Genoemd', help: 'Alleen wie je in de post tagt' }
	];
</script>

<fieldset class="flex flex-wrap gap-2">
	<legend class="sr-only">Wie ziet deze post?</legend>
	{#each options as o}
		<label
			class="cursor-pointer rounded-full border px-3 py-1 text-sm transition-colors"
			class:border-terracotta={value === o.value}
			class:bg-terracotta={value === o.value}
			class:text-white={value === o.value}
			class:border-border={value !== o.value}
			class:text-muted={value !== o.value}
			title={o.help}
		>
			<input type="radio" bind:group={value} value={o.value} name="visibility" class="sr-only" />
			{o.label}
		</label>
	{/each}
</fieldset>
