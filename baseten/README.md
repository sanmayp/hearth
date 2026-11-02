# Baseten reader deploy

Run a Meta open-weights model on Baseten. Two paths exist.
Model APIs need no deploy. Dedicated deploys need Truss.

## Path A: Model APIs (no deploy)

1. Make an account at https://www.baseten.co/ .
2. Make a personal API key at API keys. Copy it once.
3. Save it: `export BASETEN_API_KEY="PASTE_KEY"`.
4. List models and copy the exact Muse slug:
   `curl https://inference.baseten.co/v1/models -H "Authorization: Bearer $BASETEN_API_KEY"`
5. Save it: `export BASETEN_MODEL="PASTE_SLUG"`.
6. Run a capped live batch. See the repo demo script.

Full steps live in docs/BASETEN.md.

## Path B: dedicated deploy with Truss

1. Install the CLI: `pip install baseten`. Log in:
   `baseten login`. See
   https://docs.baseten.co/examples/deploy-your-first-model
2. Copy `baseten/truss-llm/` to a work dir. Set your model
   name and Hugging Face id inside `config.yaml`.
3. Push a dev deploy: `baseten model push --watch`.
4. Promote it: `baseten model deployment promote`.
5. Copy the deployment base URL. Keep it private.
   Set `BASETEN_BASE_URL` to it for the batch tool.
6. Set replicas: `min_replica: 0` sleeps free, `1` stays warm.
   See https://docs.baseten.co/deployment/autoscaling/overview

## Truss starter

`baseten/truss-llm/config.yaml` holds confirmed keys only:
model name, metadata, requirements, and resources. Extend it
per the Engine-Builder-LLM docs for large MoE builds:
https://docs.baseten.co/engines/engine-builder-llm/overview
