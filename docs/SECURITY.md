# Security

This repo holds no secrets. No keys, tokens, passwords, private hosts,
or personal contacts ship here. Demo data is fake and says so.

## Set your own keys

- `HEARTH_TOKEN`: API key for `/v1`. Set it in env at deploy.
- Bridge token: a file with mode 600. Made by
  `scripts/agent-bridge.sh setup`. Never commit it.
- `BASETEN_API_KEY`: only for live model calls. See [BASETEN](BASETEN.md).
- Signing keys for app updates: make them with `keytool` and
  `openssl` on your own host. Keep them off git.

## Safeguards in the code

- Servers refuse to start without their keys.
- Auth compares keys in constant time. Keys never log.
- Output redacts token-looking runs before it leaves the server.
- The bridge audits sends with lengths, never text.
- CORS allowlists name exact origins. Star is refused.
- Health gives no diagnoses and no drug advice. Crisis words show
  988 plus your chosen contact, then stop. Set the contact at
  deploy; the repo ships a placeholder.

## Report a problem

Open an issue on the repo. Do not paste keys or private data.
Rotate any key that ever touched a public place.
