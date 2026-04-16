/**
 * WebSocket manager for real-time DM chat.
 *
 * Connects to the API WebSocket endpoint, handles auto-reconnect with
 * exponential backoff (max 30s), and exposes methods for joining
 * conversations, sending typing indicators, and receiving events.
 */

import { browser } from '$app/environment';

// ── Types ───────────────────────────────────────────────────

export type WsMessageData = {
	uuid: string;
	sender: {
		uuid: string;
		username: string;
		display_name: string;
		avatar_url: string | null;
	};
	content: string;
	created_at: string;
	is_mine: boolean;
};

export type WsServerEvent =
	| { type: 'message'; data: WsMessageData }
	| { type: 'typing'; user: string }
	| { type: 'stop_typing'; user: string }
	| { type: 'online'; users: string[] };

type WsClientMsg =
	| { type: 'join'; conversation: string }
	| { type: 'typing'; conversation: string }
	| { type: 'stop_typing'; conversation: string };

// ── Callbacks ───────────────────────────────────────────────

export type WsCallbacks = {
	onMessage?: (msg: WsMessageData) => void;
	onTyping?: (username: string) => void;
	onStopTyping?: (username: string) => void;
	onOnline?: (users: string[]) => void;
	onConnect?: () => void;
	onDisconnect?: () => void;
};

// ── Manager ─────────────────────────────────────────────────

export class WsManager {
	private ws: WebSocket | null = null;
	private callbacks: WsCallbacks = {};
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempt = 0;
	private maxReconnectDelay = 30_000;
	private intentionalClose = false;
	private currentConversation: string | null = null;

	// Typing debounce: send typing event at most once per 3 seconds.
	private lastTypingSent = 0;
	private typingDebounceMs = 3_000;

	constructor(callbacks: WsCallbacks = {}) {
		this.callbacks = callbacks;
	}

	/** Connect to the WebSocket server. Only works in the browser. */
	connect(): void {
		if (!browser) return;
		if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
			return;
		}

		this.intentionalClose = false;

		const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		const url = `${proto}//${window.location.host}/api/ws`;

		this.ws = new WebSocket(url);

		this.ws.onopen = () => {
			this.reconnectAttempt = 0;
			this.callbacks.onConnect?.();

			// Re-join conversation if we had one before reconnect.
			if (this.currentConversation) {
				this.send({ type: 'join', conversation: this.currentConversation });
			}
		};

		this.ws.onmessage = (ev) => {
			try {
				const event = JSON.parse(ev.data) as WsServerEvent;
				switch (event.type) {
					case 'message':
						this.callbacks.onMessage?.(event.data);
						break;
					case 'typing':
						this.callbacks.onTyping?.(event.user);
						break;
					case 'stop_typing':
						this.callbacks.onStopTyping?.(event.user);
						break;
					case 'online':
						this.callbacks.onOnline?.(event.users);
						break;
				}
			} catch {
				// Ignore malformed JSON.
			}
		};

		this.ws.onclose = () => {
			this.callbacks.onDisconnect?.();
			if (!this.intentionalClose) {
				this.scheduleReconnect();
			}
		};

		this.ws.onerror = () => {
			// onerror is always followed by onclose, so reconnect
			// is handled there.
		};
	}

	/** Disconnect and stop reconnecting. */
	disconnect(): void {
		this.intentionalClose = true;
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
		this.currentConversation = null;
	}

	/** Join/subscribe to a conversation. */
	joinConversation(uuid: string): void {
		this.currentConversation = uuid;
		this.send({ type: 'join', conversation: uuid });
	}

	/** Send a typing indicator (debounced — max once per 3 seconds). */
	sendTyping(uuid: string): void {
		const now = Date.now();
		if (now - this.lastTypingSent < this.typingDebounceMs) return;
		this.lastTypingSent = now;
		this.send({ type: 'typing', conversation: uuid });
	}

	/** Send a stop-typing indicator. */
	stopTyping(uuid: string): void {
		this.lastTypingSent = 0;
		this.send({ type: 'stop_typing', conversation: uuid });
	}

	/** Whether the socket is currently open. */
	get connected(): boolean {
		return this.ws?.readyState === WebSocket.OPEN;
	}

	// ── Private ─────────────────────────────────────────────

	private send(msg: WsClientMsg): void {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(msg));
		}
	}

	private scheduleReconnect(): void {
		if (this.reconnectTimer) return;
		// Exponential backoff: 1s, 2s, 4s, 8s, ... capped at 30s.
		const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempt), this.maxReconnectDelay);
		this.reconnectAttempt++;
		this.reconnectTimer = setTimeout(() => {
			this.reconnectTimer = null;
			this.connect();
		}, delay);
	}
}
