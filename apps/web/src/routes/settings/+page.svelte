<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import Button from '$lib/components/Button.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import type { PageProps } from './$types';
	import { toasts } from '$lib/stores/toasts';

	let { data, form }: PageProps = $props();

	let avatarBusy = $state(false);
	let avatarPreview = $state<string | null>(null);

	async function onAvatarPicked(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		if (avatarPreview) URL.revokeObjectURL(avatarPreview);
		avatarPreview = URL.createObjectURL(file);
		avatarBusy = true;
		try {
			const fd = new FormData();
			fd.append('file', file);
			const res = await fetch('/api/users/me/avatar', {
				method: 'POST',
				body: fd,
				credentials: 'include'
			});
			if (!res.ok) {
				const err = (await res.json().catch(() => ({}))) as { error?: { message?: string } };
				throw new Error(err.error?.message ?? `HTTP ${res.status}`);
			}
			toasts.push('success', 'Avatar bijgewerkt');
			await invalidateAll();
		} catch (e) {
			toasts.push('error', (e as Error).message);
		} finally {
			avatarBusy = false;
			input.value = '';
		}
	}

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
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Avatar</h2>
		<div class="flex items-center gap-4">
			{#if avatarPreview}
				<img
					src={avatarPreview}
					alt="Voorbeeld"
					class="h-20 w-20 rounded-full border border-border object-cover"
				/>
			{:else}
				<Avatar url={data.user.avatar_url} name={data.user.display_name} size={80} />
			{/if}
			<label
				class="inline-flex cursor-pointer items-center gap-2 rounded-[var(--radius-button)] border border-border bg-white px-5 py-2.5 text-sm font-semibold text-ink hover:bg-border/40"
				class:opacity-60={avatarBusy}
			>
				<input
					type="file"
					class="sr-only"
					accept="image/*"
					disabled={avatarBusy}
					onchange={onAvatarPicked}
				/>
				{avatarBusy ? 'Uploaden…' : 'Andere foto kiezen'}
			</label>
		</div>
		<p class="mt-3 text-xs text-muted">
			EXIF-data (zoals GPS) wordt automatisch verwijderd voor we je foto opslaan.
		</p>
	</section>

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
