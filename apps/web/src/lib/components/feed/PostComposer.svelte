<script lang="ts">
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import Button from '$lib/components/Button.svelte';
	import VisibilityPicker from './VisibilityPicker.svelte';
	import { createPost, type Visibility, type PublicPost } from '$lib/api/posts';
	import { uploadMedia } from '$lib/api/media';
	import { toasts } from '$lib/stores/toasts';
	import type { SessionUser } from '$lib/api/core';

	type Props = {
		user: SessionUser;
		replyToUuid?: string;
		placeholder?: string;
		onPosted?: (p: PublicPost) => void;
	};
	let { user, replyToUuid, placeholder = 'Wat wil je delen?', onPosted }: Props = $props();

	let content = $state('');
	let visibility = $state<Visibility>('public');
	let mediaFiles = $state<File[]>([]);
	let previews = $state<string[]>([]);
	let posting = $state(false);

	function onFiles(e: Event) {
		const input = e.target as HTMLInputElement;
		if (!input.files) return;
		const files = Array.from(input.files).slice(0, 4 - mediaFiles.length);
		mediaFiles = [...mediaFiles, ...files];
		previews = [...previews, ...files.map((f) => URL.createObjectURL(f))];
		input.value = '';
	}

	function removeMedia(i: number) {
		URL.revokeObjectURL(previews[i]);
		mediaFiles = mediaFiles.filter((_, idx) => idx !== i);
		previews = previews.filter((_, idx) => idx !== i);
	}

	async function submit() {
		if (posting) return;
		const text = content.trim();
		if (!text && mediaFiles.length === 0) {
			toasts.push('info', 'Voeg tekst of een foto toe');
			return;
		}
		posting = true;
		try {
			// Upload media first, in parallel.
			const media_uuids = mediaFiles.length
				? (await Promise.all(mediaFiles.map((f) => uploadMedia(f)))).map((m) => m.uuid)
				: [];

			const mentions = extractMentions(text);
			const post = await createPost({
				content: text || undefined,
				media_uuids,
				visibility,
				reply_to_uuid: replyToUuid,
				mentions
			});

			// Reset + notify.
			content = '';
			previews.forEach(URL.revokeObjectURL);
			previews = [];
			mediaFiles = [];
			toasts.push('success', 'Gepost!');
			onPosted?.(post);
		} catch (e) {
			toasts.push('error', (e as Error).message);
		} finally {
			posting = false;
		}
	}

	function extractMentions(text: string): string[] {
		const re = /@([a-z0-9_]{3,30})/gi;
		const seen = new Set<string>();
		let m: RegExpExecArray | null;
		while ((m = re.exec(text)) !== null) seen.add(m[1].toLowerCase());
		return Array.from(seen);
	}
</script>

<form
	class="vonk-card"
	onsubmit={(e) => {
		e.preventDefault();
		submit();
	}}
>
	<div class="flex items-start gap-3">
		<Avatar url={user.avatar_url} name={user.display_name} size={40} />
		<textarea
			bind:value={content}
			{placeholder}
			rows={2}
			maxlength={5000}
			class="flex-1 resize-none rounded-xl border border-border bg-white px-3 py-2 text-ink placeholder:text-muted focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
		></textarea>
	</div>

	{#if previews.length}
		<div class="mt-3 grid grid-cols-2 gap-2">
			{#each previews as p, i (p)}
				<div class="relative aspect-square overflow-hidden rounded-lg bg-border">
					<img src={p} alt="preview" class="h-full w-full object-cover" />
					<button
						type="button"
						class="absolute right-1 top-1 rounded-full bg-black/60 px-2 py-1 text-xs text-white"
						onclick={() => removeMedia(i)}
					>×</button>
				</div>
			{/each}
		</div>
	{/if}

	<div class="mt-3 flex flex-wrap items-center justify-between gap-3">
		<div class="flex items-center gap-2">
			<label
				class="inline-flex cursor-pointer items-center gap-1.5 rounded-full border border-border bg-white px-3 py-1.5 text-sm font-semibold text-ink hover:bg-border/40"
				class:opacity-50={mediaFiles.length >= 4}
			>
				<input
					type="file"
					class="sr-only"
					accept="image/*"
					multiple
					disabled={mediaFiles.length >= 4}
					onchange={onFiles}
				/>
				📷 Foto
			</label>
			<span class="text-xs text-muted">{content.length}/5000</span>
		</div>
		{#if !replyToUuid}
			<VisibilityPicker bind:value={visibility} />
		{/if}
		<Button type="submit" disabled={posting}>
			{posting ? 'Posten…' : replyToUuid ? 'Antwoord' : 'Post'}
		</Button>
	</div>
</form>
