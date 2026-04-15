<script lang="ts">
	/**
	 * Modal triggered by RepostButton. Two flows:
	 *   - Simple repost (no comment) → calls repostPost, closes.
	 *   - Quote-repost → shows a small composer; submit calls repostPost
	 *     with `comment` and (optionally) picks a visibility.
	 * Closed by clicking the backdrop, Esc, or after a successful submit.
	 */
	import Button from '$lib/components/Button.svelte';
	import VisibilityPicker from './VisibilityPicker.svelte';
	import { repostPost, type Visibility, type PublicPost } from '$lib/api/posts';
	import { toasts } from '$lib/stores/toasts';

	type Props = {
		postUuid: string;
		authorUsername: string;
		open: boolean;
		onClose: () => void;
		onReposted?: (p: PublicPost) => void;
	};
	let { postUuid, authorUsername, open = $bindable(false), onClose, onReposted }: Props = $props();

	let mode = $state<'choose' | 'quote'>('choose');
	let comment = $state('');
	let visibility = $state<Visibility>('public');
	let pending = $state(false);

	function close() {
		mode = 'choose';
		comment = '';
		visibility = 'public';
		pending = false;
		onClose();
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) close();
	}

	async function simple() {
		if (pending) return;
		pending = true;
		try {
			const p = await repostPost(postUuid);
			toasts.push('success', 'Geboost!');
			onReposted?.(p);
			close();
		} catch (e) {
			toasts.push('error', (e as Error).message);
			pending = false;
		}
	}

	async function quote() {
		if (pending) return;
		const c = comment.trim();
		if (!c) {
			toasts.push('info', 'Voeg een commentaar toe');
			return;
		}
		pending = true;
		try {
			const p = await repostPost(postUuid, { comment: c, visibility });
			toasts.push('success', 'Quote gedeeld!');
			onReposted?.(p);
			close();
		} catch (e) {
			toasts.push('error', (e as Error).message);
			pending = false;
		}
	}
</script>

<svelte:window on:keydown={onKey} />

{#if open}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Repost"
		class="fixed inset-0 z-50 flex items-end justify-center bg-black/50 p-4 sm:items-center"
	>
		<!-- backdrop -->
		<button
			type="button"
			aria-label="Sluit"
			class="absolute inset-0 cursor-default"
			onclick={close}
		></button>

		<div
			class="relative w-full max-w-md rounded-[var(--radius-card)] bg-surface p-6 shadow-xl"
		>
			<header class="mb-4 flex items-center justify-between">
				<h2 class="font-display text-lg font-bold text-ink">
					{mode === 'quote' ? `Quote @${authorUsername}` : 'Repost'}
				</h2>
				<button
					type="button"
					onclick={close}
					class="rounded-full p-1 text-muted hover:bg-border"
					aria-label="Sluit"
				>✕</button>
			</header>

			{#if mode === 'choose'}
				<p class="mb-4 text-sm text-muted">Hoe wil je deze post delen?</p>
				<div class="flex flex-col gap-2">
					<Button fullWidth onclick={simple} disabled={pending}>
						🔁 Repost zonder commentaar
					</Button>
					<Button fullWidth variant="ghost" onclick={() => (mode = 'quote')}>
						✏️ Quote met commentaar
					</Button>
				</div>
			{:else}
				<textarea
					bind:value={comment}
					rows={3}
					maxlength={5000}
					placeholder="Voeg je commentaar toe…"
					class="mb-3 block w-full resize-none rounded-xl border border-border bg-surface px-3 py-2 text-ink placeholder:text-muted focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				></textarea>
				<div class="mb-4">
					<VisibilityPicker bind:value={visibility} />
				</div>
				<div class="flex items-center justify-between gap-2">
					<Button variant="ghost" onclick={() => (mode = 'choose')}>← Terug</Button>
					<Button onclick={quote} disabled={pending || !comment.trim()}>
						{pending ? 'Delen…' : 'Deel quote'}
					</Button>
				</div>
			{/if}
		</div>
	</div>
{/if}
