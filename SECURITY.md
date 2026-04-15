# Security Policy

## Reporting a Vulnerability

**Please do not open a public issue for security vulnerabilities.**

Instead, email **security@vonk.social** with:

1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if any)

We will acknowledge your report within **48 hours** and aim to resolve critical issues within **7 days**.

## Responsible Disclosure

- We ask that you give us reasonable time to fix issues before public disclosure
- We will credit you in our security advisories (unless you prefer anonymity)
- We do not pursue legal action against researchers acting in good faith

## Bug Bounty

Vonk is a donation-funded project. We cannot offer monetary bounties at this time, but we will:

- Credit you on our security acknowledgements page
- Send you Vonk swag (when we have it)
- Add you to our Hall of Fame at vonk.social/security

## Scope

In scope:
- vonk.social and all subdomains
- The Vonk API
- The Vonk mobile app
- Self-hosted instances (vulnerabilities in our code)

Out of scope:
- Third-party services (GitHub, CDN providers)
- Social engineering attacks
- Denial of service attacks
- Issues in dependencies (report those upstream, but let us know too)

## Security Practices

- Annual external penetration test (funded by donations)
- Dependency scanning via Dependabot / `cargo audit`
- All secrets encrypted at rest
- TLS 1.3 enforced
- HSTS with preload
- CSP headers on all responses
- Rate limiting on all endpoints
