<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import type { PageProps } from './$types';
	import { toasts } from '$lib/stores/toasts';

	let { data, form }: PageProps = $props();

	$effect(() => {
		if (form && 'saved' in form && form.saved) toasts.push('success', 'Opgeslagen');
		else if (form && 'error' in form && form.error)
			toasts.push('error', String((form as { error: string }).error));
	});
</script>

<svelte:head>
	<title>Instellingen — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Feed</a>
	<h1 class="mb-6 font-display text-2xl font-bold text-ink">Instellingen</h1>

	<section class="vonk-card mb-4">
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Profiel</h2>
		<form method="POST" action="?/saveProfile" class="flex flex-col gap-3">
			<label class="text-sm font-semibold text-ink">
				Weergavenaam
				<input
					type="text"
					name="display_name"
					value={data.user.display_name}
					class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				/>
			</label>
			<label class="text-sm font-semibold text-ink">
				Bio
				<textarea
					name="bio"
					rows={3}
					maxlength={500}
					class="mt-1 block w-full resize-none rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				>{data.user.bio}</textarea>
			</label>
			<div class="grid grid-cols-2 gap-3">
				<label class="text-sm font-semibold text-ink">
					Stad
					<input
						type="text"
						name="location_city"
						value={data.user.location_city ?? ''}
						class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
					/>
				</label>
				<label class="text-sm font-semibold text-ink">
					Land
					<input
						type="text"
						name="location_country"
						value={data.user.location_country ?? ''}
						class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
					/>
				</label>
			</div>
			<label class="text-sm font-semibold text-ink">
				Taal
				<select
					name="locale"
					class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				>
					<option value="nl" selected={data.user.locale === 'nl'}>Nederlands</option>
					<option value="en" selected={data.user.locale === 'en'}>English</option>
				</select>
			</label>
			<div class="self-end">
				<Button type="submit">Opslaan</Button>
			</div>
		</form>
	</section>

	<section class="vonk-card mb-4">
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Privacy</h2>
		<form method="POST" action="?/savePrivacy" class="flex items-center justify-between gap-3">
			<label class="flex items-start gap-3">
				<input
					type="checkbox"
					name="is_private"
					checked={data.user.is_private}
					class="mt-1 h-5 w-5 rounded border-border text-terracotta"
				/>
				<span>
					<span class="font-semibold text-ink">Privé-account</span>
					<span class="block text-sm text-muted">
						Mensen moeten toestemming vragen om je posts en stories te zien.
					</span>
				</span>
			</label>
			<Button type="submit">Opslaan</Button>
		</form>
	</section>

	<section class="vonk-card">
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Sessie</h2>
		<form method="POST" action="?/logoutEverywhere">
			<Button type="submit" variant="ghost">Afmelden</Button>
		</form>
	</section>
</main>

<Toast />
