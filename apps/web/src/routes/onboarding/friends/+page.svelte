<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? 'nl');
</script>

<form method="POST" action="?/done" class="vonk-card flex flex-col gap-6">
	<h2 class="font-display text-xl font-bold text-ink">{t('friends.title', locale)}</h2>
	<p class="text-muted">{t('friends.help', locale)}</p>

	<div class="flex justify-end">
		<Button type="submit">{t('friends.done', locale)} →</Button>
	</div>
</form>
