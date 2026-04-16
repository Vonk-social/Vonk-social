<script lang="ts">
	import { onMount, onDestroy, tick } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import { fetchMessages, sendMessage } from '$lib/api/dm';
	import type { DmMessage } from '$lib/api/dm';
	import { WsManager } from '$lib/ws';
	import type { WsMessageData } from '$lib/ws';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let messages = $state<DmMessage[]>([...data.messages].reverse());
	let cursor = $state<string | null>(data.cursor);
	let hasMore = $state(data.hasMore);
	let loadingOlder = $state(false);

	let messageInput = $state('');
	let sending = $state(false);
	let chatContainer: HTMLDivElement | undefined = $state();

	// WebSocket state.
	let typingUser = $state<string | null>(null);
	let typingTimeout: ReturnType<typeof setTimeout> | null = null;
	let wsManager: WsManager | null = null;

	// Our own username — used to filter out our own typing/message events.
	const myUsername = data.user?.username ?? '';

	async function scrollToBottom() {
		await tick();
		if (chatContainer) {
			chatContainer.scrollTop = chatContainer.scrollHeight;
		}
	}

	onMount(() => {
		scrollToBottom();

		// Connect WebSocket.
		wsManager = new WsManager({
			onMessage: (msg: WsMessageData) => {
				// Skip our own messages — we already appended them optimistically.
				if (msg.sender.username === myUsername) return;

				const dmMsg: DmMessage = {
					uuid: msg.uuid,
					sender: msg.sender,
					content: msg.content,
					created_at: msg.created_at,
					is_mine: false
				};

				// Deduplicate: check if already present (from HTTP response).
				if (!messages.some((m) => m.uuid === dmMsg.uuid)) {
					messages = [...messages, dmMsg];
					scrollToBottom();
				}

				// Clear typing indicator when a message arrives.
				if (typingUser === msg.sender.username) {
					typingUser = null;
				}
			},
			onTyping: (username: string) => {
				if (username === myUsername) return;
				typingUser = username;
				// Auto-clear after 5 seconds.
				if (typingTimeout) clearTimeout(typingTimeout);
				typingTimeout = setTimeout(() => {
					typingUser = null;
				}, 5000);
			},
			onStopTyping: (username: string) => {
				if (typingUser === username) {
					typingUser = null;
				}
			}
		});

		wsManager.connect();
		wsManager.joinConversation(data.conversationUuid);
	});

	onDestroy(() => {
		if (wsManager) {
			wsManager.disconnect();
			wsManager = null;
		}
		if (typingTimeout) {
			clearTimeout(typingTimeout);
		}
	});

	async function loadOlder() {
		if (!hasMore || loadingOlder || !cursor) return;
		loadingOlder = true;
		try {
			const res = await fetchMessages(data.conversationUuid, { cursor, limit: 50 });
			const older = res.data.reverse();
			// Preserve scroll position: measure before, insert, restore offset.
			const prevHeight = chatContainer?.scrollHeight ?? 0;
			messages = [...older, ...messages];
			cursor = res.cursor;
			hasMore = res.has_more;
			await tick();
			if (chatContainer) {
				chatContainer.scrollTop = chatContainer.scrollHeight - prevHeight;
			}
		} catch {
			// silently fail
		} finally {
			loadingOlder = false;
		}
	}

	async function handleSend() {
		const text = messageInput.trim();
		if (!text || sending) return;
		sending = true;

		// Stop typing indicator.
		if (wsManager) wsManager.stopTyping(data.conversationUuid);

		try {
			const msg = await sendMessage(data.conversationUuid, text);
			messages = [...messages, msg];
			messageInput = '';
			await scrollToBottom();
		} catch {
			// silently fail
		} finally {
			sending = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSend();
		}
	}

	function handleInput() {
		if (wsManager && messageInput.trim()) {
			wsManager.sendTyping(data.conversationUuid);
		}
	}

	function formatTime(iso: string): string {
		const d = new Date(iso);
		return d.toLocaleTimeString('nl-BE', { hour: '2-digit', minute: '2-digit' });
	}

	function formatDateSep(iso: string): string {
		const d = new Date(iso);
		const today = new Date();
		const yesterday = new Date();
		yesterday.setDate(yesterday.getDate() - 1);
		if (d.toDateString() === today.toDateString()) return 'Vandaag';
		if (d.toDateString() === yesterday.toDateString()) return 'Gisteren';
		return d.toLocaleDateString('nl-BE', { day: 'numeric', month: 'long' });
	}

	function shouldShowDate(idx: number): boolean {
		if (idx === 0) return true;
		const prev = new Date(messages[idx - 1].created_at).toDateString();
		const curr = new Date(messages[idx].created_at).toDateString();
		return prev !== curr;
	}
</script>

<svelte:head>
	<title>{data.otherUser?.display_name ?? 'Chat'} — Vonk</title>
</svelte:head>

<div class="flex h-[100dvh] flex-col">
	<!-- Header -->
	<header class="flex items-center gap-3 border-b border-border bg-white px-4 py-3">
		<a href="/dm" class="text-muted hover:text-ink" aria-label="Terug">
			<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M15 19l-7-7 7-7" />
			</svg>
		</a>
		{#if data.otherUser}
			<Avatar url={data.otherUser.avatar_url} name={data.otherUser.display_name} size={36} />
			<div class="min-w-0 flex-1">
				<p class="font-bold text-ink">{data.otherUser.display_name}</p>
				<p class="text-xs text-muted">@{data.otherUser.username}</p>
			</div>
		{:else}
			<p class="font-bold text-ink">Chat</p>
		{/if}
	</header>

	<!-- Messages -->
	<div
		bind:this={chatContainer}
		class="flex-1 overflow-y-auto px-4 py-4"
	>
		{#if hasMore}
			<div class="mb-4 text-center">
				<button
					type="button"
					class="text-sm font-semibold text-terracotta hover:underline"
					disabled={loadingOlder}
					onclick={loadOlder}
				>
					{loadingOlder ? 'Laden...' : 'Oudere berichten laden'}
				</button>
			</div>
		{/if}

		{#each messages as msg, idx (msg.uuid)}
			{#if shouldShowDate(idx)}
				<div class="my-4 text-center text-xs text-muted">{formatDateSep(msg.created_at)}</div>
			{/if}
			<div class="mb-2 flex {msg.is_mine ? 'justify-end' : 'justify-start'}">
				{#if !msg.is_mine}
					<div class="mr-2 mt-auto flex-shrink-0">
						<Avatar url={msg.sender.avatar_url} name={msg.sender.display_name} size={28} />
					</div>
				{/if}
				<div
					class="max-w-[75%] rounded-2xl px-4 py-2 {msg.is_mine
						? 'bg-terracotta text-white'
						: 'border border-border bg-surface text-ink'}"
				>
					<p class="whitespace-pre-wrap break-words text-sm">{msg.content}</p>
					<p class="mt-0.5 text-right text-[10px] {msg.is_mine ? 'text-white/70' : 'text-muted'}">
						{formatTime(msg.created_at)}
					</p>
				</div>
			</div>
		{/each}

		{#if messages.length === 0}
			<div class="flex h-full items-center justify-center text-center text-muted">
				<p>Stuur het eerste bericht!</p>
			</div>
		{/if}

		{#if typingUser}
			<div class="mb-2 flex items-center gap-2 pl-1">
				<div class="flex gap-0.5">
					<span class="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-muted [animation-delay:0ms]"></span>
					<span class="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-muted [animation-delay:150ms]"></span>
					<span class="inline-block h-1.5 w-1.5 animate-bounce rounded-full bg-muted [animation-delay:300ms]"></span>
				</div>
				<span class="text-xs text-muted">{typingUser} is aan het typen...</span>
			</div>
		{/if}
	</div>

	<!-- Input bar — sits above the BottomNav -->
	<div class="border-t border-border bg-white px-4 py-3" style="padding-bottom: calc(0.75rem + env(safe-area-inset-bottom, 0px)); margin-bottom: 4rem;">
		<div class="flex items-end gap-2">
			<textarea
				class="max-h-32 min-h-[40px] flex-1 resize-none rounded-2xl border border-border bg-surface px-4 py-2 text-sm text-ink placeholder:text-muted focus:border-terracotta focus:outline-none"
				placeholder="Typ een bericht..."
				rows="1"
				bind:value={messageInput}
				onkeydown={handleKeydown}
				oninput={handleInput}
			></textarea>
			<button
				type="button"
				class="flex h-10 w-10 items-center justify-center rounded-full bg-terracotta text-white transition-colors hover:bg-terracotta-dark disabled:opacity-50"
				disabled={!messageInput.trim() || sending}
				onclick={handleSend}
				aria-label="Verstuur"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
					<path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
				</svg>
			</button>
		</div>
	</div>
</div>
