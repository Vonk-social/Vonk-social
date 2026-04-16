<script lang="ts">
	import { goto } from '$app/navigation';
	import CameraCapture from '$lib/components/camera/CameraCapture.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import VisibilityPicker from '$lib/components/feed/VisibilityPicker.svelte';
	import Button from '$lib/components/Button.svelte';
	import { uploadMedia } from '$lib/api/media';
	import { createPost, type Visibility } from '$lib/api/posts';
	import { sendSnap } from '$lib/api/snaps';
	import { getOrCreateKeypair, encryptFor } from '$lib/e2ee';
	import { apiFetch } from '$lib/api/core';
	import { toasts } from '$lib/stores/toasts';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	type Stage = 'capture' | 'choose' | 'sending';
	let stage = $state<Stage>('capture');
	let blob = $state<Blob | null>(null);
	let previewUrl = $state<string | null>(null);
	let caption = $state('');
	let visibility = $state<Visibility>('public');
	let snapRecipient = $state('');

	function onCapture(b: Blob) {
		blob = b;
		if (previewUrl) URL.revokeObjectURL(previewUrl);
		previewUrl = URL.createObjectURL(b);
		stage = 'choose';
	}

	function retake() {
		if (previewUrl) URL.revokeObjectURL(previewUrl);
		previewUrl = null;
		blob = null;
		stage = 'capture';
	}

	async function sendAs(kind: 'post' | 'story' | 'snap') {
		if (!blob) return;
		stage = 'sending';
		try {
			const media = await uploadMedia(blob);
			if (kind === 'post') {
				await createPost({
					content: caption || undefined,
					media_uuids: [media.uuid],
					visibility,
					post_type: 'post'
				});
				toasts.push('success', 'Post gedeeld!');
				goto('/home');
			} else if (kind === 'story') {
				await createPost({
					content: caption || undefined,
					media_uuids: [media.uuid],
					visibility,
					post_type: 'story'
				});
				toasts.push('success', 'Story staat online (24u)');
				goto('/home');
			} else if (kind === 'snap') {
				const recipient = snapRecipient.trim().replace(/^@/, '');
				if (!recipient) {
					toasts.push('error', 'Vul een gebruikersnaam in');
					stage = 'choose';
					return;
				}

				// Try E2EE: fetch recipient's public key + encrypt a random snap key.
				let envelope: { ephemeral_pubkey: string; nonce: string; ciphertext: string } | undefined;
				try {
					const profileRes = await apiFetch(`/api/users/${encodeURIComponent(recipient)}`);
					if (profileRes.ok) {
						const profile = (await profileRes.json()).data as { public_key?: string | null };
						if (profile.public_key) {
							// Ensure we have our own keypair registered.
							await getOrCreateKeypair();
							// Encrypt a random 32-byte snap key with the recipient's X25519 pubkey.
							const snapKey = crypto.getRandomValues(new Uint8Array(32));
							envelope = await encryptFor(profile.public_key, snapKey);
						}
					}
				} catch {
					// E2EE not possible — send plaintext (v0). Not fatal.
				}

				await sendSnap({
					to_username: recipient,
					media_uuid: media.uuid,
					view_policy: 'view_once',
					...(envelope ?? {})
				});
				toasts.push('success', `Snap verstuurd aan @${snapRecipient}`);
				goto('/dm');
			}
		} catch (e) {
			toasts.push('error', (e as Error).message);
			stage = 'choose';
		}
	}
</script>

<svelte:head>
	<title>Vonk — Camera</title>
</svelte:head>

{#if stage === 'capture'}
	<div class="h-dvh">
		<CameraCapture {onCapture} />
	</div>
{:else}
	<main class="mx-auto flex min-h-screen max-w-md flex-col px-4 py-6">
		<header class="mb-4 flex items-center justify-between">
			<button
				type="button"
				class="rounded-full bg-white px-3 py-1 text-sm font-semibold text-ink shadow hover:bg-border/40"
				onclick={retake}
			>← Opnieuw</button>
			<h1 class="font-display text-xl font-bold text-ink">Waar gaat dit heen?</h1>
		</header>

		{#if previewUrl}
			<div class="mb-4 overflow-hidden rounded-[var(--radius-card)]">
				<img src={previewUrl} alt="voorbeeld" class="block w-full" />
			</div>
		{/if}

		<textarea
			bind:value={caption}
			placeholder="Bijschrift (optioneel)…"
			rows={2}
			maxlength={500}
			class="mb-4 w-full resize-none rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
		></textarea>

		<div class="mb-4">
			<p class="mb-2 text-sm font-semibold text-muted">Wie ziet dit?</p>
			<VisibilityPicker bind:value={visibility} />
		</div>

		<div class="flex flex-col gap-2">
			<Button fullWidth onclick={() => sendAs('post')} disabled={stage === 'sending'}>
				📝 Post in je feed
			</Button>
			<Button
				fullWidth
				variant="amber"
				onclick={() => sendAs('story')}
				disabled={stage === 'sending'}
			>
				📖 Story — 24 uur zichtbaar
			</Button>

			<div class="my-3 border-t border-border"></div>
			<p class="text-sm font-semibold text-muted">Of snap naar iemand specifiek:</p>
			<div class="flex gap-2">
				<input
					bind:value={snapRecipient}
					placeholder="@gebruikersnaam"
					class="flex-1 rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				/>
				<Button onclick={() => sendAs('snap')} disabled={stage === 'sending' || !snapRecipient}>
					⚡ Snap
				</Button>
			</div>
		</div>
	</main>
{/if}

<Toast />
