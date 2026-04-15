<script lang="ts">
	import GoogleButton from '$lib/components/GoogleButton.svelte';
	import Card from '$lib/components/Card.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';
	let { data }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? 'nl');
</script>

<svelte:head>
	<title>{t('login.title', locale)} — Vonk</title>
</svelte:head>

<main class="mx-auto flex min-h-screen max-w-md flex-col items-center justify-center px-6 py-16">
	<Card class="w-full">
		<h1 class="font-display text-3xl font-bold text-ink">{t('login.title', locale)}</h1>
		<p class="mt-3 text-muted">{t('login.subtitle', locale)}</p>

		<div class="mt-8">
			<GoogleButton label={t('login.google', locale)} />
		</div>

		<p class="mt-6 text-center text-sm text-muted">{t('login.more_providers', locale)}</p>
	</Card>
</main>
