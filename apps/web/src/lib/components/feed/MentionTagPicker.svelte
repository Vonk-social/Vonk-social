<script lang="ts">
	/**
	 * Autocomplete popup for `@mentions` and `#hashtags` inside a <textarea>.
	 *
	 * The popup floats directly under the textarea (simple, robust positioning
	 * — we deliberately do NOT try to anchor to the caret pixel-coordinate
	 * because that requires mirror-layout hacks and breaks on wrap). Up/Down
	 * to navigate, Enter/Tab to accept, Escape to dismiss.
	 */
	import { onMount } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import { searchUsers, searchTags, type UserCard, type TagSuggestion } from '$lib/api/users';

	type Props = {
		/** Bound textarea element — the picker listens to its input/keydown. */
		textarea: HTMLTextAreaElement | null;
		/** Bound value of the textarea; picker edits it on accept. */
		value: string;
		onValueChange: (v: string) => void;
	};
	let { textarea, value, onValueChange }: Props = $props();

	type Trigger = { char: '@' | '#'; start: number; query: string };
	let trigger = $state<Trigger | null>(null);
	let users = $state<UserCard[]>([]);
	let tags = $state<TagSuggestion[]>([]);
	let active = $state(0);
	let debounceId: ReturnType<typeof setTimeout> | undefined;

	const results = $derived(
		trigger?.char === '@'
			? users.map((u) => ({ kind: 'user' as const, key: u.username, user: u }))
			: tags.map((t) => ({ kind: 'tag' as const, key: t.tag, tag: t }))
	);

	function detectTrigger(): Trigger | null {
		if (!textarea) return null;
		const pos = textarea.selectionStart ?? 0;
		// Scan backwards from the caret to find a word-start trigger.
		let i = pos - 1;
		let query = '';
		while (i >= 0) {
			const ch = value[i];
			if (ch === '@' || ch === '#') {
				// Must be at start-of-input or preceded by whitespace/punct.
				const before = i > 0 ? value[i - 1] : ' ';
				if (/[\s(\[{]/.test(before) || i === 0) {
					return { char: ch as '@' | '#', start: i, query };
				}
				return null;
			}
			if (/[A-Za-z0-9_]/.test(ch)) {
				query = ch + query;
				i -= 1;
			} else {
				return null;
			}
		}
		return null;
	}

	async function refresh() {
		const t = detectTrigger();
		trigger = t;
		if (!t) {
			users = [];
			tags = [];
			return;
		}
		const q = t.query;
		clearTimeout(debounceId);
		debounceId = setTimeout(async () => {
			try {
				if (t.char === '@') {
					users = q.length >= 1 ? await searchUsers(q) : [];
				} else {
					tags = q.length >= 1 ? await searchTags(q) : [];
				}
				active = 0;
			} catch {
				users = [];
				tags = [];
			}
		}, 150);
	}

	function accept(which?: number) {
		if (!trigger || !textarea) return;
		const idx = which ?? active;
		const r = results[idx];
		if (!r) return;
		const token = r.kind === 'user' ? `@${r.user.username}` : r.tag.tag;
		const before = value.slice(0, trigger.start);
		const afterStart = trigger.start + 1 + trigger.query.length; // past the triggered segment
		const after = value.slice(afterStart);
		// Insert a space after the token so the next keystroke starts a new word.
		const next = `${before}${token} ${after}`;
		onValueChange(next);
		// Move caret right after the inserted token.
		const caret = before.length + token.length + 1;
		queueMicrotask(() => {
			if (textarea) {
				textarea.focus();
				textarea.setSelectionRange(caret, caret);
			}
		});
		trigger = null;
		users = [];
		tags = [];
	}

	function onKeydown(e: KeyboardEvent) {
		if (!trigger || results.length === 0) return;
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			active = (active + 1) % results.length;
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			active = (active - 1 + results.length) % results.length;
		} else if (e.key === 'Enter' || e.key === 'Tab') {
			e.preventDefault();
			accept();
		} else if (e.key === 'Escape') {
			e.preventDefault();
			trigger = null;
		}
	}

	onMount(() => {
		if (!textarea) return;
		const onInput = () => refresh();
		const onSelect = () => refresh();
		const onKd = (e: Event) => onKeydown(e as KeyboardEvent);
		textarea.addEventListener('input', onInput);
		textarea.addEventListener('click', onSelect);
		textarea.addEventListener('keyup', onSelect);
		textarea.addEventListener('keydown', onKd);
		return () => {
			textarea?.removeEventListener('input', onInput);
			textarea?.removeEventListener('click', onSelect);
			textarea?.removeEventListener('keyup', onSelect);
			textarea?.removeEventListener('keydown', onKd);
		};
	});
</script>

{#if trigger && results.length > 0}
	<ul
		class="mt-2 max-h-64 overflow-y-auto rounded-xl border border-border bg-white shadow-lg"
		role="listbox"
		aria-label={trigger.char === '@' ? 'Mentions' : 'Hashtags'}
	>
		{#each results as r, i (r.key)}
			<li>
				<button
					type="button"
					class="flex w-full items-center gap-3 px-3 py-2 text-left transition-colors"
					class:bg-border={i === active}
					onmouseenter={() => (active = i)}
					onclick={() => accept(i)}
				>
					{#if r.kind === 'user'}
						<Avatar url={r.user.avatar_url} name={r.user.display_name} size={32} />
						<span class="flex flex-col leading-tight">
							<span class="font-semibold text-ink">{r.user.display_name}</span>
							<span class="text-xs text-muted">@{r.user.username}</span>
						</span>
					{:else}
						<span
							class="flex h-8 w-8 items-center justify-center rounded-full bg-amber/20 text-amber"
						>#</span>
						<span class="flex flex-col leading-tight">
							<span class="font-semibold text-ink">{r.tag.tag}</span>
							<span class="text-xs text-muted">{r.tag.count}× gebruikt</span>
						</span>
					{/if}
				</button>
			</li>
		{/each}
	</ul>
{/if}
