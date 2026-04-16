// Vonk service worker — Web Push dispatcher.
//
// Keeps scope tiny: we don't do offline caching in v1 (SvelteKit already
// does SSR → hydration). This SW only exists so the browser can wake us
// for incoming push notifications.

self.addEventListener('install', () => {
	self.skipWaiting();
});

self.addEventListener('activate', (event) => {
	event.waitUntil(self.clients.claim());
});

self.addEventListener('push', (event) => {
	let data = {};
	try {
		data = event.data ? event.data.json() : {};
	} catch {
		data = { title: 'Vonk', body: event.data ? event.data.text() : '' };
	}
	const title = data.title || 'Vonk';
	const options = {
		body: data.body || '',
		icon: '/icons/vonk-192.png',
		badge: '/icons/vonk-badge.png',
		tag: data.tag || 'vonk',
		data: { url: data.url || '/' }
	};
	event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener('notificationclick', (event) => {
	event.notification.close();
	const url = (event.notification.data && event.notification.data.url) || '/';
	event.waitUntil(
		self.clients.matchAll({ type: 'window' }).then((list) => {
			for (const c of list) {
				if (c.url === url && 'focus' in c) return c.focus();
			}
			return self.clients.openWindow(url);
		})
	);
});
