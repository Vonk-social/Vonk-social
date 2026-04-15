<script lang="ts">
	import { untrack } from 'svelte';
	import Input from '$lib/components/Input.svelte';
	import Button from '$lib/components/Button.svelte';
	import { checkUsername } from '$lib/api';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? 'nl');

	let username = $state('');
	// Initial value only — the user edits it, the form action persists it.
	let display_name = $state(untrack(() => data.user?.display_name ?? ''));

	let status = $state<null | { available: boolean; reason?: string }>(null);
	let checking = $state(false);

	// Debounced availability check.
	let debounceId: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const val = username.trim();
		clearTimeout(debounceId);
		if (val.length < 3) {
			status = null;
			return;
		}
		checking = true;
		debounceId = setTimeout(async () => {
			try {
				status = await checkUsername(val);
			} finally {
				checking = false;
			}
		}, 250);
	});

	const statusMessage = $derived.by(() => {
		if (!status || checking) return null;
		if (status.available) return { success: t('username.available', locale) };
		if (status.reason === 'invalid_format') return { error: t('username.invalid', locale) };
		if (status.reason === 'reserved') return { error: t('username.reserved', locale) };
		return { error: t('username.taken', locale) };
	});
</script>

<form method="POST" class="vonk-card flex flex-col gap-6">
	<Input
		bind:value={username}
		name="username"
		label={t('username.label', locale)}
		placeholder="bv. dimitry"
		autocomplete="off"
		help={statusMessage ? undefined : t('username.help', locale)}
		error={statusMessage?.error}
		success={statusMessage?.success}
	/>

	<Input
		bind:value={display_name}
		name="display_name"
		label="Weergavenaam"
		placeholder="Dimitry Smagghe"
		autocomplete="name"
	/>

	{#if form?.error}
		<p class="text-sm text-terracotta">{form.error}</p>
	{/if}

	<div class="flex justify-end">
		<Button
			type="submit"
			disabled={checking || !status?.available || username.length < 3}
		>
			{t('username.continue', locale)} →
		</Button>
	</div>
</form>
