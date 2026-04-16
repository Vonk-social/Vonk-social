<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? 'nl');
</script>

<div class="vonk-card flex flex-col gap-6">
	<h2 class="font-display text-xl font-bold text-ink">{t('friends.title', locale)}</h2>
	<p class="text-muted">{t('friends.help', locale)}</p>

	<div class="flex flex-col gap-3 sm:flex-row">
		<a href="/invite" class="flex-1">
			<Button type="button" fullWidth>
				✉️ {t('friends.invite', locale)}
			</Button>
		</a>
		<a href="/discover" class="flex-1">
			<Button type="button" variant="ghost" fullWidth>
				🔍 {t('friends.discover', locale)}
			</Button>
		</a>
	</div>

	<form method="POST" action="?/done">
		<div class="flex justify-end">
			<Button type="submit" variant="ghost">{t('friends.done', locale)} →</Button>
		</div>
	</form>
</div>
