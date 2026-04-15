-- ============================================================
-- Vonk — dev-only seed users
-- ============================================================
-- Purpose: populate the DEVELOPMENT database with a set of plausible users
-- so `/discover`, `/home` feed and `/u/...` pages show real content.
-- These users cannot log in (no `user_auth_providers` rows) — they're
-- ambient population for the current account to interact with.
--
-- Safe to re-run: every insert has `ON CONFLICT DO NOTHING`.
--
-- Run with:
--   docker exec -i vonk-social-vonk-db-1 psql -U vonk vonk \
--     < packages/db/seed/dev-users.sql
-- ============================================================

BEGIN;

-- ── Users ────────────────────────────────────────────────────
INSERT INTO users (uuid, username, display_name, bio, location_city, location_country, locale, is_private, onboarding_completed_at, created_at)
VALUES
  ('11111111-1111-1111-1111-111111111001', 'anouk',      'Anouk De Smet',     'Grafisch ontwerper uit Gent. Houdt van risoprint, filterkoffie en trage fietsritten.', 'Gent',       'België',    'nl', false, now() - interval '20 days', now() - interval '20 days'),
  ('11111111-1111-1111-1111-111111111002', 'mathias',    'Mathias Janssens',  'Podcastmaker. Praat te veel over klimaat, muziek en Belgische chocolade.',              'Antwerpen',  'België',    'nl', false, now() - interval '18 days', now() - interval '18 days'),
  ('11111111-1111-1111-1111-111111111003', 'sofia',      'Sofía García',      'Documentary photographer. Currently shooting a series on European night trains.',      'Madrid',     'España',    'en', false, now() - interval '15 days', now() - interval '15 days'),
  ('11111111-1111-1111-1111-111111111004', 'lars',       'Lars Nordström',    'Software engineer. Building privacy tools and teaching his kids to code.',              'Stockholm',  'Sverige',   'en', false, now() - interval '12 days', now() - interval '12 days'),
  ('11111111-1111-1111-1111-111111111005', 'fatima',     'Fatima El-Hassan',  'Urban gardener. Writing about community gardens and food sovereignty.',                 'Rotterdam',  'Nederland', 'nl', false, now() - interval '10 days', now() - interval '10 days'),
  ('11111111-1111-1111-1111-111111111006', 'pieter',     'Pieter Van Looy',   'Boer op pensioen. Fermenteert alles wat fermenteerbaar is. Vraag me naar miso.',       'Wetteren',   'België',    'nl', false, now() - interval '8 days',  now() - interval '8 days'),
  ('11111111-1111-1111-1111-111111111007', 'marieke',    'Marieke Vermeulen', 'Kinderboekenillustrator + mama van twee. Post veel inktvlekken en LEGO-chaos.',         'Brugge',     'België',    'nl', true,  now() - interval '6 days',  now() - interval '6 days'),
  ('11111111-1111-1111-1111-111111111008', 'jakob',      'Jakob Ferenczi',    'Cook and recipe writer. Cooking my way through every regional cuisine of Europe.',     'Wenen',      'Österreich','en', false, now() - interval '4 days',  now() - interval '4 days')
ON CONFLICT (username) DO NOTHING;

-- ── Make the first user (you) follow a few of them ───────────
-- Assumes your real account lives at `username = 'dimitry'`; adjust if yours differs.
INSERT INTO follows (follower_id, following_id, status, created_at)
SELECT you.id, them.id, 'active', now() - interval '1 day'
FROM users you
JOIN users them ON them.username = ANY(ARRAY['anouk', 'mathias', 'sofia', 'jakob'])
WHERE you.username = 'dimitry'
ON CONFLICT DO NOTHING;

-- ── Mutual follows between dummies — so "misschien ken je" (mutual-follows heuristic) has signal ──
-- Everyone follows Anouk (popular). Sofia + Lars + Fatima follow each other.
INSERT INTO follows (follower_id, following_id, status, created_at)
SELECT f.id, t.id, 'active', now() - interval '2 days'
FROM users f, users t
WHERE f.username IN ('mathias', 'sofia', 'lars', 'fatima', 'pieter', 'jakob')
  AND t.username = 'anouk'
  AND f.id <> t.id
ON CONFLICT DO NOTHING;

INSERT INTO follows (follower_id, following_id, status, created_at)
SELECT f.id, t.id, 'active', now() - interval '2 days'
FROM users f, users t
WHERE f.username IN ('sofia', 'lars', 'fatima')
  AND t.username IN ('sofia', 'lars', 'fatima')
  AND f.id <> t.id
ON CONFLICT DO NOTHING;

-- marieke is private — anouk + mathias have pending follow requests so you can test the accept flow
INSERT INTO follows (follower_id, following_id, status, created_at)
SELECT f.id, t.id, 'pending', now() - interval '3 hours'
FROM users f, users t
WHERE f.username IN ('anouk', 'mathias')
  AND t.username = 'marieke'
ON CONFLICT DO NOTHING;

-- ── Some public posts so your feed isn't empty ───────────────
INSERT INTO posts (uuid, user_id, content, post_type, visibility, created_at)
SELECT gen_random_uuid(), u.id, txt, 'post', 'public', ts
FROM (VALUES
  ('anouk',   'Eerste risoprint-test op ochtendlicht. De magenta is 𝘴𝘤ℎ𝘦𝘦𝘧, maar dat is exact waarom ik het graag zie. 🎨',    now() - interval '2 days'),
  ('mathias', 'Aflevering 42 staat online: in gesprek met een energiecoöperatie in de Kempen. Koffie + KwH.',                      now() - interval '1 day 4 hours'),
  ('sofia',   'Night train from Brussels → Vienna. 10 hours. Best sleep I had in a month.',                                        now() - interval '1 day 2 hours'),
  ('lars',    'Shipped: a tiny tool that strips EXIF from photos before upload. Open source. Link in next post.',                  now() - interval '18 hours'),
  ('fatima',  'Gemeentelijke tuin op het dak van de parkeergarage. Dit is de derde oogst aubergines deze maand.',                  now() - interval '10 hours'),
  ('pieter',  'Mijn miso is na 8 maanden klaar. Kleur als oude leer, smaak als 𝘶𝘮𝘢𝘮𝘪-𝘸𝘰𝘭𝘬𝘦𝘯. Wie komt proeven?',                    now() - interval '6 hours'),
  ('jakob',   'Today in the kitchen: the 14th version of goulash. Still not right. Still delicious.',                              now() - interval '2 hours'),
  ('anouk',   'Kleine vraag voor iedereen die hier ook zit: wat is jullie favoriete lokale koffiebrander? Ik zoek iets nieuws.',   now() - interval '40 minutes')
) AS v(uname, txt, ts)
JOIN users u ON u.username = v.uname;

-- ── One story per two of the people you'll follow so the tray shows up ──
INSERT INTO posts (uuid, user_id, content, post_type, visibility, expires_at, created_at)
SELECT gen_random_uuid(), u.id, txt, 'story', 'public', now() + interval '22 hours', now() - interval '2 hours'
FROM (VALUES
  ('sofia', 'Espresso + printer proofs.'),
  ('jakob', 'Third try at the croissants.')
) AS v(uname, txt)
JOIN users u ON u.username = v.uname;

COMMIT;

-- ── Sanity check ─────────────────────────────────────────────
SELECT
  (SELECT COUNT(*) FROM users WHERE username != 'dimitry') AS dummy_users,
  (SELECT COUNT(*) FROM follows)                           AS follow_edges,
  (SELECT COUNT(*) FROM posts WHERE post_type = 'post')    AS posts,
  (SELECT COUNT(*) FROM posts WHERE post_type = 'story')   AS stories;
