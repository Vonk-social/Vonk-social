/**
 * Web Push subscription helpers (browser-side).
 *
 * The service worker lives at `/sw.js` and is the one that receives
 * pushes. This module handles the registration / subscription /
 * upload-to-backend dance.
 */

async function fetchVapidKey(): Promise<string | null> {
	try {
		const res = await fetch('/api/push/vapid-public-key', { credentials: 'include' });
		if (!res.ok) return null;
		const body = (await res.json()) as { data: { public_key: string } };
		return body.data.public_key;
	} catch {
		return null;
	}
}

function urlBase64ToUint8Array(base64: string): BufferSource {
	const padding = '='.repeat((4 - (base64.length % 4)) % 4);
	const normalized = (base64 + padding).replace(/-/g, '+').replace(/_/g, '/');
	const raw = atob(normalized);
	const buffer = new ArrayBuffer(raw.length);
	const out = new Uint8Array(buffer);
	for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
	return out;
}

function toBase64Url(buf: ArrayBuffer | null): string {
	if (!buf) return '';
	const bytes = new Uint8Array(buf);
	let bin = '';
	for (const b of bytes) bin += String.fromCharCode(b);
	return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export async function isSupported(): Promise<boolean> {
	return (
		typeof window !== 'undefined' &&
		'serviceWorker' in navigator &&
		'PushManager' in window &&
		Notification.permission !== 'denied'
	);
}

export async function currentSubscription(): Promise<PushSubscription | null> {
	if (!('serviceWorker' in navigator)) return null;
	const reg = await navigator.serviceWorker.getRegistration();
	if (!reg) return null;
	return await reg.pushManager.getSubscription();
}

export async function subscribe(): Promise<boolean> {
	if (!(await isSupported())) return false;

	const perm = await Notification.requestPermission();
	if (perm !== 'granted') return false;

	const vapid = await fetchVapidKey();
	if (!vapid) return false;

	let reg = await navigator.serviceWorker.getRegistration();
	if (!reg) reg = await navigator.serviceWorker.register('/sw.js');

	let sub = await reg.pushManager.getSubscription();
	if (!sub) {
		sub = await reg.pushManager.subscribe({
			userVisibleOnly: true,
			applicationServerKey: urlBase64ToUint8Array(vapid)
		});
	}

	const p256dh = toBase64Url(sub.getKey('p256dh'));
	const auth = toBase64Url(sub.getKey('auth'));

	const res = await fetch('/api/push/subscriptions', {
		method: 'POST',
		credentials: 'include',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({
			endpoint: sub.endpoint,
			p256dh,
			auth,
			user_agent: navigator.userAgent
		})
	});
	return res.ok;
}

export async function unsubscribe(): Promise<boolean> {
	const sub = await currentSubscription();
	if (!sub) return true;
	await sub.unsubscribe();
	return true;
}
