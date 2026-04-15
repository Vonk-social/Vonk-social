<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { toasts } from '$lib/stores/toasts';

	type Props = { onCapture: (blob: Blob) => void };
	let { onCapture }: Props = $props();

	let video = $state<HTMLVideoElement | undefined>();
	let facingMode = $state<'user' | 'environment'>('user');
	let stream: MediaStream | null = null;
	let starting = $state(false);
	let error = $state<string | null>(null);

	async function start() {
		stop();
		starting = true;
		error = null;
		try {
			stream = await navigator.mediaDevices.getUserMedia({
				video: { facingMode, width: { ideal: 1080 }, height: { ideal: 1920 } },
				audio: false
			});
			if (video) {
				video.srcObject = stream;
				await video.play();
			}
		} catch (e) {
			error = (e as Error).message || 'Kon camera niet openen';
			toasts.push('error', error);
		} finally {
			starting = false;
		}
	}

	function stop() {
		stream?.getTracks().forEach((t) => t.stop());
		stream = null;
	}

	async function flip() {
		facingMode = facingMode === 'user' ? 'environment' : 'user';
		await start();
	}

	async function shoot() {
		if (!video || !video.videoWidth) return;
		const canvas = document.createElement('canvas');
		canvas.width = video.videoWidth;
		canvas.height = video.videoHeight;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		// Mirror the front-cam preview so the output matches what the user sees.
		if (facingMode === 'user') {
			ctx.translate(canvas.width, 0);
			ctx.scale(-1, 1);
		}
		ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
		canvas.toBlob(
			(blob) => {
				if (blob) onCapture(blob);
			},
			'image/webp',
			0.85
		);
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.key === ' ' || e.code === 'Space') {
			e.preventDefault();
			shoot();
		}
	}

	onMount(start);
	onDestroy(stop);
</script>

<svelte:window on:keydown={onKeyDown} />

<div class="relative flex h-full w-full flex-col overflow-hidden bg-black">
	<div class="relative flex-1">
		<video
			bind:this={video}
			class="h-full w-full object-cover"
			class:scale-x-[-1]={facingMode === 'user'}
			playsinline
			muted
			autoplay
		></video>

		{#if error}
			<div class="absolute inset-0 flex items-center justify-center p-8 text-center text-white">
				<div>
					<p class="font-semibold">📷 Camera niet beschikbaar</p>
					<p class="mt-2 text-sm opacity-80">{error}</p>
					<p class="mt-3 text-xs opacity-60">Tip: sta camera-toegang toe in je browser.</p>
				</div>
			</div>
		{/if}
	</div>

	<!-- Controls -->
	<div class="flex items-center justify-around bg-black/80 py-6">
		<button
			type="button"
			class="rounded-full p-3 text-white hover:bg-white/10"
			onclick={flip}
			aria-label="Wissel camera"
		>🔄</button>

		<button
			type="button"
			class="relative flex h-20 w-20 items-center justify-center rounded-full bg-white focus-visible:outline-4 focus-visible:outline-white"
			onclick={shoot}
			aria-label="Foto nemen"
			disabled={!!error || starting}
		>
			<span class="h-16 w-16 rounded-full border-4 border-ink"></span>
		</button>

		<a
			href="/home"
			class="rounded-full p-3 text-white hover:bg-white/10"
			aria-label="Annuleer"
		>✕</a>
	</div>
</div>
