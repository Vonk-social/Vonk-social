# VONK — Financieel Model & Transparantie

**Principe: Elke cent is zichtbaar. Wat we niet nodig hebben, gaat naar wie het wél nodig heeft.**

---

## Het Model in Één Zin

> Donaties dekken de hosting. Wat overblijft, gaat jaarlijks naar goede doelen voor wereldvrede en gezondheid. De boekhouding is 100% openbaar, real-time, op `vonk.social/open`.

---

## 1. Inkomstenstromen

Vonk heeft bewust maar drie bronnen van inkomsten:

| Bron | Hoe | Verwacht |
|---|---|---|
| **GitHub Sponsors** | Maandelijkse of eenmalige donaties via GitHub | Hoofdstroom |
| **Buy Me a Coffee** | Laagdrempelig, eenmalig | Impulsdonaties |
| **In-app donatie** | Vrijwillige bijdrage vanuit de app (€1/€3/€5/vrij) | Nadat platform draait |

Wat Vonk **nooit** zal doen:
- Advertenties tonen
- Gegevens verkopen
- Premium features achter een paywall zetten
- Investeerders aannemen
- Geld aannemen van partijen die invloed willen op het platform

---

## 2. Kostenstructuur

### Vaste kosten (maandelijks)

Elke euro die binnenkomt, gaat eerst naar het draaiende houden van het platform:

| Categorie | Wat valt hieronder | Richtbedrag |
|---|---|---|
| **Hosting** | Servers, database, CDN, DNS | Variabel per fase |
| **Domein & certificaten** | vonk.social, SSL | ~€25/jaar |
| **Diensten** | Email (Resend), monitoring (Grafana) | ~€10–50/maand |
| **Security audit** | Jaarlijkse externe pentest | ~€2.000–5.000/jaar |
| **Transactiekosten** | GitHub/Stripe/BMAC fees (~5–8%) | % van donaties |

### Wat NIET in de kosten zit

- Salarissen — Vonk is 100% vrijwilligerswerk
- Marketing — groei is organisch of mond-tot-mond
- Kantoorkosten — er is geen kantoor
- Legal — pro bono of gedekt door community-juristen

Als er ooit een punt komt waarop part-time betaalde maintainers nodig zijn, wordt dat eerst als RFC aan de community voorgelegd en transparant begroot.

---

## 3. Het Overschot: Goede Doelen

### De Regel

```
Maandelijkse donaties
    minus  Werkelijke hostingkosten
    minus  Operationele kosten
    minus  Reserve (3 maanden hosting vooruit)
    ─────────────────────────────────────
    =  OVERSCHOT → Jaarlijkse uitkering aan goede doelen
```

### De Reserve

Vonk houdt altijd een buffer aan van **3 maanden operationele kosten**. Dit voorkomt dat een slechte donatiemaand het platform offline haalt. Alles boven die buffer is overschot.

### De Goede Doelen

Het overschot wordt **jaarlijks** in december uitgekeerd aan organisaties die bijdragen aan twee pijlers:

**Pijler 1 — Wereldvrede**
- Conflictbemiddeling en diplomatie
- Onderwijs in conflictgebieden
- Persvrijheid en onafhankelijke journalistiek
- Mensenrechtenorganisaties

**Pijler 2 — Gezondheid**
- Toegang tot gezondheidszorg in ontwikkelingslanden
- Medisch onderzoek (open access)
- Mentale gezondheid en preventie
- Schoon water en sanitatie

### Selectiecriteria voor goede doelen

Niet elk goed doel komt in aanmerking. Vonk hanteert strenge criteria:

| Criterium | Waarom |
|---|---|
| **Onafhankelijk** | Geen politieke of religieuze affiliatie |
| **Transparant** | Publiceert eigen jaarrekening en impactrapport |
| **Efficiënt** | Minimaal 80% van donaties gaat naar het doel, niet naar overhead |
| **Geen wapenindustrie** | Geen enkele link met defensie of wapenhandel |
| **Verifieerbaar** | Geregistreerd als erkend goed doel in hun jurisdictie |

### Stemming

De Vonk-community stemt jaarlijks (november) over de verdeling:

1. **Core team** stelt een shortlist van 5–10 organisaties voor
2. **Geverifieerde donateurs** (iedereen die dat jaar gedoneerd heeft) mogen stemmen
3. Elke donateur krijgt **100 punten** om te verdelen over de shortlist
4. Uitkering in december, proportioneel aan de stemuitslag

Voorbeeld:

```
Jaarlijks overschot: €4.200

Stemuitslag:
  Artsen Zonder Grenzen          28%  →  €1.176
  International Crisis Group     22%  →  €924
  Wikimedia Foundation           18%  →  €756
  WaterAid                       17%  →  €714
  Press Freedom Foundation       15%  →  €630
                                       ────────
                                       €4.200
```

---

## 4. Transparante Boekhouding: vonk.social/open

### Real-time Dashboard

Elke bezoeker (ook niet-ingelogd) kan op `vonk.social/open` zien:

```
╔══════════════════════════════════════════════════════╗
║  VONK OPEN BOEKHOUDING                              ║
║                                                      ║
║  Deze maand (april 2026)                             ║
║  ┌──────────────────────────────────────────┐        ║
║  │  Donaties ontvangen          €347,00     │        ║
║  │  Hostingkosten              -€142,50     │        ║
║  │  Diensten                    -€12,00     │        ║
║  │  Transactiekosten            -€24,29     │        ║
║  │  ──────────────────────────────────────  │        ║
║  │  Netto deze maand            €168,21     │        ║
║  └──────────────────────────────────────────┘        ║
║                                                      ║
║  Reserve                                             ║
║  ┌──────────────────────────────────────────┐        ║
║  │  Doelbedrag (3 maanden)      €463,50     │        ║
║  │  Huidig saldo                €412,80     │        ║
║  │  ██████████████████████░░░░  89%         │        ║
║  └──────────────────────────────────────────┘        ║
║                                                      ║
║  Goede Doelen Pot 2026                               ║
║  ┌──────────────────────────────────────────┐        ║
║  │  Opgebouwd dit jaar         €1.847,30    │        ║
║  │  Uitkering in december 2026              │        ║
║  │  Stemming opent november 2026            │        ║
║  └──────────────────────────────────────────┘        ║
║                                                      ║
║  [Bekijk alle transacties]  [Download CSV]           ║
╚══════════════════════════════════════════════════════╝
```

### Wat er zichtbaar is

| Gegeven | Detail | Privacy |
|---|---|---|
| **Elke donatie** | Bedrag, datum, bron (GitHub/BMAC/in-app) | Naam donateur optioneel (anoniem mogelijk) |
| **Elke uitgave** | Bedrag, datum, leverancier, categorie, factuur-PDF | Volledig openbaar |
| **Maandtotalen** | In/uit/netto per maand | Volledig openbaar |
| **Reserve-stand** | Hoeveel buffer er is | Volledig openbaar |
| **Goede doelen pot** | Opgebouwd bedrag + historiek | Volledig openbaar |
| **Jaarlijkse uitkering** | Welk doel, hoeveel, betalingsbewijs | Volledig openbaar |

### Technische implementatie

De boekhouding draait niet in een apart systeem — het is onderdeel van de Vonk-database:

```sql
CREATE TABLE finances (
    id              BIGSERIAL PRIMARY KEY,
    date            DATE NOT NULL,
    type            TEXT NOT NULL CHECK (type IN (
                        'donation_github',
                        'donation_bmac', 
                        'donation_inapp',
                        'expense_hosting',
                        'expense_service',
                        'expense_domain',
                        'expense_security',
                        'expense_transaction_fee',
                        'charity_payout'
                    )),
    amount_cents    INTEGER NOT NULL,      -- positief = inkomst, negatief = uitgave
    currency        TEXT DEFAULT 'EUR',
    description     TEXT NOT NULL,
    donor_name      TEXT,                  -- NULL = anoniem
    donor_public    BOOLEAN DEFAULT false, -- mag naam getoond worden?
    recipient       TEXT,                  -- leverancier of goed doel
    receipt_url     TEXT,                  -- link naar factuur/betalingsbewijs
    category        TEXT,
    created_at      TIMESTAMPTZ DEFAULT now()
);

-- Maandelijkse samenvatting (materialized view, ververst dagelijks)
CREATE MATERIALIZED VIEW finance_monthly AS
SELECT 
    date_trunc('month', date) AS month,
    SUM(CASE WHEN amount_cents > 0 THEN amount_cents ELSE 0 END) AS income_cents,
    SUM(CASE WHEN amount_cents < 0 THEN amount_cents ELSE 0 END) AS expense_cents,
    SUM(amount_cents) AS net_cents,
    COUNT(*) FILTER (WHERE type LIKE 'donation_%') AS donation_count
FROM finances
GROUP BY 1
ORDER BY 1 DESC;

-- Goede doelen uitkeringen
CREATE TABLE charity_payouts (
    id              BIGSERIAL PRIMARY KEY,
    year            INTEGER NOT NULL,
    charity_name    TEXT NOT NULL,
    charity_url     TEXT,
    amount_cents    INTEGER NOT NULL,
    vote_percentage NUMERIC(5,2),
    payment_proof   TEXT,                  -- URL naar betalingsbewijs
    paid_at         DATE
);

-- Stemming
CREATE TABLE charity_votes (
    id              BIGSERIAL PRIMARY KEY,
    year            INTEGER NOT NULL,
    user_id         BIGINT REFERENCES users(id),
    charity_name    TEXT NOT NULL,
    points          INTEGER NOT NULL CHECK (points >= 0 AND points <= 100),
    voted_at        TIMESTAMPTZ DEFAULT now(),
    UNIQUE(year, user_id, charity_name)
);

-- Constraint: max 100 punten per user per jaar
-- (enforced in applicatielaag)
```

### API Endpoints (openbaar, geen auth nodig)

```
GET /api/open/summary
GET /api/open/transactions?year=2026&month=4
GET /api/open/transactions.csv?year=2026
GET /api/open/reserve
GET /api/open/charity/current-year
GET /api/open/charity/history
GET /api/open/charity/vote-results?year=2025
```

---

## 5. Juridische Structuur

### Stichting Vonk (of VZW Vonk)

Om donaties correct te verwerken en goede doelen uit te keren, heeft Vonk een rechtspersoon nodig:

| Optie | Voordeel | Nadeel |
|---|---|---|
| **Belgische VZW** | Dicht bij huis, eenvoudig oprichten (~€150), vertrouwd | Belgische boekhoudplicht, BTW-registratie bij omzet |
| **Nederlandse Stichting** | Internationaler imago, eenvoudige structuur | Buitenlands, KvK-registratie |
| **Duitse e.V.** | Grote open-source community in DE | Complexere oprichting |

**Aanbeveling**: Start als **Belgische VZW**. Simpelste route, je kent het systeem, en het past bij de Europese identiteit van Vonk.

VZW-vereisten:
- Minimum 2 oprichters
- Statuten publiceren in Belgisch Staatsblad (~€150)
- Doel: "Het beheren en ontwikkelen van het open-source sociaal platform Vonk, en het uitkeren van overschotten aan goede doelen gericht op wereldvrede en gezondheid"
- Jaarrekening neerleggen bij griffie (vereenvoudigd schema voor kleine VZW's)
- Geen winstuitkering aan leden (by design — dat is precies wat we willen)

### Fiscaal voordeel voor donateurs

Als de VZW erkend wordt als **instelling die giften mag ontvangen** (Art. 145/33 WIB), kunnen Belgische donateurs hun gift fiscaal aftrekken (45% belastingvermindering). Dit vereist erkenning door de FOD Financiën — een traject van enkele maanden, maar enorm waardevol voor donatiemotivatie.

---

## 6. Communicatie naar Donateurs

### Maandelijkse Update

Elke eerste maandag van de maand publiceert Vonk een kort financieel overzicht:

- Op `vonk.social/open` (automatisch)
- Als blogpost op `vonk.social/blog`
- Via GitHub Discussions
- Optioneel: email naar donateurs die dit willen

### Jaarverslag (december)

Een publiek jaarverslag met:

1. Financieel overzicht (inkomsten, uitgaven, reserve, overschot)
2. Goede doelen: wie heeft wat gekregen + betalingsbewijzen
3. Platform statistieken (users, posts, uptime — geen privacy-gevoelige data)
4. Technisch overzicht (wat is er gebouwd)
5. Vooruitblik volgend jaar
6. Dankwoord aan donateurs (met naam, tenzij anoniem)

---

## 7. Safeguards

### Tegen misbruik

- **Geen individu heeft toegang tot de gelden** — twee handtekeningen nodig voor elke uitgave boven €100
- **Bankrekening op naam van de VZW**, niet op persoonsnaam
- **Alle uitgaven hebben een factuur** die publiek zichtbaar is
- **Community kan alarmeren** — als iets niet klopt, is dat zichtbaar op /open

### Tegen afhankelijkheid van één donateur

- Geen enkele donateur mag meer dan 30% van het jaarbudget uitmaken
- Bij overschrijding: actief andere donateurs werven
- Dit voorkomt dat één partij invloed kan uitoefenen via geld

### Tegen scope creep

- De VZW-statuten leggen het doel vast
- Wijziging van het doel vereist statutenwijziging (Algemene Vergadering + Staatsblad)
- De goede doelen pijlers (wereldvrede + gezondheid) staan in de statuten

---

## Samenvatting

```
    €€€ Donaties komen binnen
         │
         ▼
    ┌─────────────┐
    │  HOSTING    │ ← Eerst dit dekken
    │  + KOSTEN   │
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │  RESERVE    │ ← 3 maanden buffer opbouwen
    │  (3 mnd)    │
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │  OVERSCHOT  │ ← Alles hierboven
    └──────┬──────┘
           │
     November: community stemt
           │
           ▼
    ┌─────────────┐
    │  GOEDE      │ ← December: uitkering
    │  DOELEN     │    Wereldvrede + Gezondheid
    │  🕊️  🏥     │    Publiek betalingsbewijs
    └─────────────┘
```

*Elke stap is openbaar. Elke euro is traceerbaar. Vonk is van iedereen.*
