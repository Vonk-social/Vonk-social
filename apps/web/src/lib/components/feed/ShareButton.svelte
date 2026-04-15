<script lang="ts">
	import { toasts } from '$lib/stores/toasts';

	type Props = { postUuid: string; authorUsername: string };
	let { postUuid, authorUsername }: Props = $props();

	async function share() {
		const url = `${window.location.origin}/post/${postUuid}`;
		const title = `Post van @${authorUsername} op Vonk`;
		// Use the native share sheet when available (mobile + modern macOS);
		// clipboard fallback everywhere else.
		if (typeof navigator !== 'undefined' && typeof navigator.share === 'function') {
			try {
				await navigator.share({ url, title });
				return;
			} catch (e) {
				// User cancelled → ignore silently. Any other error falls
				// through to the clipboard fallback.
				if ((e as DOMException).name === 'AbortError') return;
			}
		}
		try {
			await navigator.clipboard.writeText(url);
			toasts.push('success', 'Link gekopieerd');
		} catch {
			toasts.push('error', 'Kon link niet kopiëren');
		}
	}
</script>

<button
	type="button"
	onclick={share}
	aria-label="Deel deze post"
	class="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-semibold text-muted transition-colors hover:bg-border hover:text-terracotta"
>
	<svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="2">
		<path
			stroke-linecap="round"
			stroke-linejoin="round"
			d="M4 12v7a1 1 0 001 1h14a1 1 0 001-1v-7M16 6l-4-4-4 4M12 2v14"
		/>
	</svg>
	<span class="sr-only">Delen</span>
</button>
