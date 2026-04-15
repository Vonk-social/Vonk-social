<script lang="ts">
	import { onDestroy } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import { markStoryViewed } from '$lib/api/posts';
	import type { StoryGroup, StoryItem } from '$lib/api/feed';

	type Props = { group: StoryGroup; onClose: () => void };
	let { group, onClose }: Props = $props();

	const DURATION_MS = 5000;

	let index = $state(0);
	let progress = $state(0);
	let paused = $state(false);
	let timer: ReturnType<typeof setInterval> | null = null;
	let lastTick = $state(Date.now());

	function start() {
		stop();
		lastTick = Date.now();
		timer = setInterval(tick, 50);
	}

	function stop() {
		if (timer) clearInterval(timer);
		timer = null;
	}

	function tick() {
		if (paused) {
			lastTick = Date.now();
			return;
		}
		const now = Date.now();
		progress += (now - lastTick) / DURATION_MS;
		lastTick = now;
		if (progress >= 1) next();
	}

	function next() {
		progress = 0;
		if (index + 1 >= group.items.length) {
			onClose();
			return;
		}
		index += 1;
		markViewed(group.items[index]);
	}

	function prev() {
		progress = 0;
		if (index === 0) return;
		index -= 1;
	}

	function markViewed(item: StoryItem) {
		markStoryViewed(item.uuid);
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.key === 'ArrowRight') next();
		else if (e.key === 'ArrowLeft') prev();
		else if (e.key === 'Escape') onClose();
	}

	$effect(() => {
		// re-start whenever group/index changes
		void group.author.uuid;
		markViewed(group.items[0]);
		start();
		return () => stop();
	});

	onDestroy(() => stop());

	const item = $derived(group.items[index]);
	const mediaSrc = $derived(
		item?.media[0]?.variants.full ??
			item?.media[0]?.variants.medium ??
			item?.media[0]?.variants.thumb ??
			''
	);
</script>

<svelte:window on:keydown={onKeyDown} />

<div
	role="dialog"
	aria-modal="true"
	aria-label="Stories van {group.author.display_name}"
	class="fixed inset-0 z-50 flex items-center justify-center bg-black"
>
	<!-- Progress bars -->
	<div class="absolute inset-x-4 top-4 z-10 flex gap-1">
		{#each group.items as _, i}
			<div class="h-1 flex-1 overflow-hidden rounded-full bg-white/20">
				<div
					class="h-full bg-white"
					style="width: {i < index ? 100 : i === index ? Math.min(100, progress * 100) : 0}%;"
				></div>
			</div>
		{/each}
	</div>

	<!-- Header -->
	<div class="absolute inset-x-4 top-8 z-10 flex items-center justify-between">
		<div class="flex items-center gap-2 text-white">
			<Avatar url={group.author.avatar_url} name={group.author.display_name} size={32} />
			<span class="font-semibold">{group.author.display_name}</span>
		</div>
		<button
			type="button"
			class="rounded-full bg-white/10 px-3 py-1 text-white hover:bg-white/20"
			onclick={onClose}
			aria-label="Sluit"
		>✕</button>
	</div>

	<!-- Media -->
	<div class="relative h-full w-full max-w-md">
		{#if mediaSrc}
			<img
				src={mediaSrc}
				alt=""
				class="h-full w-full select-none object-contain"
				draggable="false"
			/>
		{/if}

		<!-- Tap zones -->
		<button
			type="button"
			class="absolute inset-y-0 left-0 w-1/3 cursor-default"
			aria-label="Vorige"
			onclick={prev}
			onpointerdown={() => (paused = true)}
			onpointerup={() => (paused = false)}
		></button>
		<button
			type="button"
			class="absolute inset-y-0 right-0 w-1/3 cursor-default"
			aria-label="Volgende"
			onclick={next}
			onpointerdown={() => (paused = true)}
			onpointerup={() => (paused = false)}
		></button>
	</div>
</div>
