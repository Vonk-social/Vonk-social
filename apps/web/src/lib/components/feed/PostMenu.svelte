<script lang="ts">
	/**
	 * Kebab (⋯) menu on a PostCard. Only rendered when the current viewer is
	 * the post's author. Contains destructive and author-only actions that
	 * shouldn't clutter the main action row: pin/unpin, delete.
	 */
	import { deletePost, pinPost, unpinPost } from '$lib/api/posts';
	import { toasts } from '$lib/stores/toasts';

	type Props = {
		postUuid: string;
		pinned: boolean;
		onPinnedChange?: (pinned: boolean) => void;
		onDeleted?: () => void;
	};
	let { postUuid, pinned, onPinnedChange, onDeleted }: Props = $props();

	let open = $state(false);
	let pending = $state(false);

	function close() {
		open = false;
	}

	function onDocClick(e: MouseEvent) {
		if (!open) return;
		const target = e.target as HTMLElement;
		if (!target.closest?.('[data-post-menu]')) open = false;
	}

	async function togglePin() {
		if (pending) return;
		pending = true;
		try {
			if (pinned) {
				await unpinPost(postUuid);
				onPinnedChange?.(false);
				toasts.push('success', 'Losgemaakt');
			} else {
				await pinPost(postUuid);
				onPinnedChange?.(true);
				toasts.push('success', 'Vastgemaakt op je profiel');
			}
			close();
		} catch (e) {
			toasts.push('error', (e as Error).message);
		} finally {
			pending = false;
		}
	}

	async function doDelete() {
		if (pending) return;
		if (!confirm('Deze post verwijderen? Dit kan niet ongedaan worden gemaakt.')) return;
		pending = true;
		try {
			await deletePost(postUuid);
			onDeleted?.();
			toasts.push('success', 'Verwijderd');
			close();
		} catch (e) {
			toasts.push('error', (e as Error).message);
		} finally {
			pending = false;
		}
	}
</script>

<svelte:window on:click={onDocClick} />

<div data-post-menu class="relative">
	<button
		type="button"
		onclick={(e) => {
			e.stopPropagation();
			open = !open;
		}}
		aria-haspopup="menu"
		aria-expanded={open}
		aria-label="Meer acties"
		class="rounded-full p-1.5 text-muted hover:bg-border"
	>⋯</button>

	{#if open}
		<ul
			role="menu"
			class="absolute right-0 z-10 mt-1 min-w-44 overflow-hidden rounded-xl border border-border bg-surface shadow-lg"
		>
			<li role="none">
				<button
					type="button"
					role="menuitem"
					onclick={togglePin}
					disabled={pending}
					class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm hover:bg-border disabled:opacity-60"
				>
					{pinned ? '📌 Losmaken' : '📌 Vastmaken op profiel'}
				</button>
			</li>
			<li role="none">
				<button
					type="button"
					role="menuitem"
					onclick={doDelete}
					disabled={pending}
					class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm text-terracotta hover:bg-border disabled:opacity-60"
				>
					🗑 Verwijder
				</button>
			</li>
		</ul>
	{/if}
</div>
