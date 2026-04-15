<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();
	const locale = $derived(data.user?.locale ?? 'nl');

	let preview = $state<string | null>(null);
	let fileName = $state<string | null>(null);
	let uploading = $state(false);

	function onFileChange(e: Event) {
		const input = e.target as HTMLInputElement;
		const f = input.files?.[0];
		if (!f) return;
		fileName = f.name;
		if (preview) URL.revokeObjectURL(preview);
		preview = URL.createObjectURL(f);
	}
</script>

<form
	method="POST"
	action="?/upload"
	enctype="multipart/form-data"
	class="vonk-card flex flex-col gap-6"
	onsubmit={() => (uploading = true)}
>
	<h2 class="font-display text-xl font-bold text-ink">{t('avatar.title', locale)}</h2>
	<p class="text-muted">{t('avatar.help', locale)}</p>

	<div class="flex items-center gap-6">
		<div
			class="h-28 w-28 flex-shrink-0 overflow-hidden rounded-full border-2 border-dashed border-border bg-cream"
		>
			{#if preview}
				<img src={preview} alt="Voorbeeld van je foto" class="h-full w-full object-cover" />
			{/if}
		</div>

		<div>
			<label
				class="inline-flex cursor-pointer items-center gap-2 rounded-[var(--radius-button)]
				       border border-border bg-white px-5 py-2.5 text-sm font-semibold text-ink
				       hover:bg-border/40"
			>
				<input
					type="file"
					name="file"
					accept="image/png,image/jpeg,image/webp"
					class="sr-only"
					onchange={onFileChange}
				/>
				{preview ? t('avatar.change', locale) : t('avatar.choose', locale)}
			</label>
			{#if fileName}
				<p class="mt-2 text-sm text-muted">{fileName}</p>
			{/if}
		</div>
	</div>

	{#if form?.error}
		<p class="text-sm text-terracotta">{form.error}</p>
	{/if}

	<div class="flex flex-wrap justify-between gap-3">
		<Button variant="ghost" type="submit" formaction="?/skip">
			{t('avatar.skip', locale)}
		</Button>
		<Button type="submit" disabled={!preview || uploading}>
			{uploading ? t('avatar.uploading', locale) : `${t('avatar.continue', locale)} →`}
		</Button>
	</div>
</form>
