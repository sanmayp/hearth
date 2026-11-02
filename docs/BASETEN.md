# Baseten worked example

Hearth reads public text through a model on Baseten. This doc shows
each step. Judges with no key run mock mode instead. No step here
needs a key until step 5.

![Baseten steps: account, library, deploy, URL, env, run](images/baseten.png)

## What it does

`tools/baseten-reader/reader.mjs` reads one text file and asks a
model for a short summary. It calls an OpenAI-compatible endpoint.
It sends public text only, never private data. It prints no keys.

## Step by step

1. Create a Baseten account at [baseten.co](https://www.baseten.co/).
   Follow their docs at [docs.baseten.co](https://docs.baseten.co/).
2. Open the model library. Pick a Meta open-weights model:
   Muse Spark Open for large agentic work, or Muse Glimmer
   for a small fast reader. Use the exact model name the
   library shows for your call.
3. Deploy it, or use the shared Model APIs endpoint with no
   deploy step. Baseten serves an OpenAI-compatible API.
4. Copy your endpoint URL and make an API key in API keys.
5. Set env in your shell (never in the repo):
   `BASETEN_API_KEY`, `BASETEN_MODEL`, and `BASETEN_BASE_URL`
   when you use your own deploy. Default base is the public
   Model APIs URL `https://inference.baseten.co/v1`.
6. Run the reader on the sample filing:
   `node tools/baseten-reader/reader.mjs fixtures/filing.txt`.
   It prints at most 3 short sentences.

## Mock mode

Run with `--mock` to skip the network and the key:

```bash
node tools/baseten-reader/reader.mjs --mock fixtures/filing.txt
```

Mock prints a fixed extract plus a banner. `scripts/demo.sh`
runs this mode. Use it in reviews and CI.

## Notes

- The reader caps input at 2000 chars and output at 200 tokens.
- Live calls cost inference on your own Baseten account.
- The same env pattern fits other partners. See [PARTNERS](PARTNERS.md).
