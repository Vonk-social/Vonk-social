<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	const locale = $derived(data.user.locale);
	const welcome = $derived(t('home.welcome', locale).replace('{name}', data.user.display_name));
</script>

<svelte:head>
	<title>Vonk — {data.user.username}</title>
</svelte:head>

<main class="mx-auto flex min-h-screen max-w-2xl flex-col px-6 py-10">
	<header class="flex items-center justify-between">
		<div class="flex items-center gap-4">
			{#if data.user.avatar_url}
				<img
					src={data.user.avatar_url}
					alt={data.user.display_name}
					class="h-14 w-14 rounded-full border border-border object-cover"
				/>
			{:else}
				<div
					class="flex h-14 w-14 items-center justify-center rounded-full bg-terracotta text-lg font-bold text-white"
				>
					{data.user.display_name.slice(0, 1)}
				</div>
			{/if}
			<div>
				<p class="font-display text-xl font-bold text-ink">{welcome}</p>
				<p class="text-sm text-muted">@{data.user.username}</p>
			</div>
		</div>

		<form method="POST" action="?/logout">
			<Button type="submit" variant="ghost">{t('home.logout', locale)}</Button>
		</form>
	</header>

	<section class="mt-10">
		<Card>
			<p class="text-muted">{t('home.empty', locale)}</p>
		</Card>
	</section>
</main>
