# Partners

Hearth builds on Meta models and runs them on hackathon partner
platforms. This file names each partner and the adapter pattern.
[docs/BASETEN.md](BASETEN.md) holds the full worked example.

## Meta

- Muse Spark and Muse Code: the developer agent behind this repo.
- Meta Model API: hosted Muse Spark series, Muse Wonder,
  Muse Image, and SAM 3.1.
- Open weights: Muse Spark Open for agentic coding work,
  Muse Glimmer for small fast local use.

## Baseten

- Runs Meta open-weights models behind OpenAI-compatible endpoints.
- Main path in this repo. See [BASETEN](BASETEN.md).
- Env: `BASETEN_API_KEY`, `BASETEN_MODEL`, `BASETEN_BASE_URL`.

## Fireworks AI

- Runs open-weights models behind one OpenAI-compatible base URL.
  See [docs.fireworks.ai](https://docs.fireworks.ai/).
- Same adapter shape as the Baseten reader: base URL plus key
  plus the model name from the Fireworks model list.
- Suggested env: `FIREWORKS_API_KEY`, `FIREWORKS_MODEL`.
  Keep mock mode for judges with no key.

## Modal

- Runs open-weights models on serverless GPUs behind Endpoints
  you own. See [modal.com](https://modal.com/).
- Serve the weights (for example with vLLM, which speaks the
  OpenAI-compatible wire protocol), then point the reader at your URL.
- Suggested env: `MODAL_MODEL_URL`, plus your endpoint auth.
  Keep mock mode for judges with no key.
