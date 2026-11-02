# Modal reader deploy

Serve a Meta open-weights model on Modal GPUs. The server speaks
the open chat-completions wire protocol. Proxy tokens guard all calls. It sleeps
after five idle minutes, so idle time bills nothing.

## Prereqs

- A Modal account. Install the SDK: `pip install modal`.
- A Hugging Face token with access to your gated weights.
  Save it: `modal secret create huggingface HF_TOKEN="$HF_TOKEN"`.
- Pick a model id from Hugging Face. Use a 30B-class model on
  A100-80GB. Bigger weights need bigger GPUs.

## Deploy (6 steps)

1. Export the model id. No default ships here on purpose.
   `export HEARTH_MODEL_ID="PASTE_HF_MODEL_ID"`
2. Optional pin: `export HEARTH_MODEL_REVISION="PASTE_SHA"`.
3. Optional GPU: `export HEARTH_GPU="A100-80GB"`. Default fits
   30B-class weights. See GPU classes at
   https://modal.com/docs/guide/gpu
4. Deploy: `modal deploy modal/serve.py`. Copy the server URL
   it prints. Keep the URL private. Never commit it.
5. Make a proxy token: `modal workspace proxy-tokens create`.
   Save the id and the secret.
6. Point the batch tool at it:
   `export MODAL_MODEL_URL="PASTE_SERVER_URL"` and
   `export MODAL_MODEL="hearth-reader"` plus the proxy parts.
   Then run a capped live batch. See the repo demo script.

## Shape notes

- Image, vLLM flags, volumes, and timeouts mirror the official
  Modal vLLM example: https://modal.com/docs/examples/vllm_inference
- `--enforce-eager` favors fast boots over peak speed. Good for
  demos with scale-to-zero. Drop it for steady traffic.
- Auth stays on. Requests need
  `Authorization: Bearer <id>.<secret>`.
  See https://modal.com/docs/guide/servers
