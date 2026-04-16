<script lang="ts">
	import { t } from '$lib/i18n';
	import VonkLogo from '$lib/components/VonkLogo.svelte';
	import LanguageSwitcher from '$lib/components/ui/LanguageSwitcher.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? data.locale ?? 'nl');

	const sections = [1, 2, 3, 4, 5, 6, 7, 8] as const;
</script>

<svelte:head>
	<title>Vonk — {t('terms.title', locale)}</title>
</svelte:head>

<div class="absolute top-4 right-4 z-10">
	<LanguageSwitcher current={locale} />
</div>

<article class="mx-auto max-w-3xl px-4 pt-16 pb-20">
	<header class="mb-10 flex flex-col items-center text-center">
		<a href="/" aria-label="Vonk">
			<VonkLogo size={48} alt="Vonk" />
		</a>
		<h1 class="mt-6 font-display text-3xl font-extrabold text-ink md:text-4xl">
			{t('terms.title', locale)}
		</h1>
		<p class="mt-2 text-sm text-muted">{t('terms.updated', locale)}</p>
	</header>

	<div class="vonk-card mb-6">
		<p class="text-muted">{t('terms.intro', locale)}</p>
	</div>

	{#each sections as n}
		<section class="vonk-card mb-4">
			<h2 class="font-display text-lg font-bold text-ink">
				{t(`terms.section${n}_title`, locale)}
			</h2>
			<p class="mt-2 text-muted">
				{t(`terms.section${n}_body`, locale)}
			</p>
		</section>
	{/each}

	<div class="mt-10 text-center">
		<a
			href="/"
			class="text-sm font-medium text-muted underline decoration-border underline-offset-4 hover:text-ink"
		>
			{t('terms.back', locale)}
		</a>
	</div>
</article>
