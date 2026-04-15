<script lang="ts">
	import type { Snippet } from 'svelte';
	type Props = {
		class?: string;
		children: Snippet;
	};
	const { class: cls = '', children }: Props = $props();
</script>

<div class="vonk-card {cls}">
	{@render children()}
</div>
