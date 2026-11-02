"""Hearth reader on Modal. vLLM serves a Meta open-weights model.

OpenAI-compatible. Auth stays on: proxy tokens guard all calls.
Scale-to-zero: the server sleeps after five idle minutes.
Shape follows the official Modal vLLM example.
Docs: https://modal.com/docs/examples/vllm_inference
"""

import json
import os
import subprocess

import modal

MODEL_ID = os.environ.get("HEARTH_MODEL_ID", "")
if not MODEL_ID:
    raise RuntimeError("Set HEARTH_MODEL_ID to a Hugging Face model id first.")

MODEL_REVISION = os.environ.get("HEARTH_MODEL_REVISION", "")
GPU = os.environ.get("HEARTH_GPU", "A100-80GB")
VLLM_PORT = 8000

vllm_image = (
    modal.Image.from_registry("nvidia/cuda:12.9.0-devel-ubuntu22.04", add_python="3.12")
    .entrypoint([])
    .uv_pip_install("vllm==0.21.0")
    .env({"HF_XET_HIGH_PERFORMANCE": "1"})
)

hf_cache_vol = modal.Volume.from_name("huggingface-cache", create_if_missing=True)
vllm_cache_vol = modal.Volume.from_name("vllm-cache", create_if_missing=True)

app = modal.App("hearth-reader")


@app.server(
    image=vllm_image,
    gpu=GPU,
    scaledown_window=5 * 60,
    startup_timeout=10 * 60,
    volumes={
        "/root/.cache/huggingface": hf_cache_vol,
        "/root/.cache/vllm": vllm_cache_vol,
    },
    port=VLLM_PORT,
    target_concurrency=4,
    secrets=[modal.Secret.from_name("huggingface")],
)
class Server:
    @modal.enter()
    def start(self):
        cmd = [
            "vllm",
            "serve",
            MODEL_ID,
            "--served-model-name",
            "hearth-reader",
            "--host",
            "0.0.0.0",
            "--port",
            str(VLLM_PORT),
            "--enforce-eager",
            "--tensor-parallel-size",
            "1",
            "--limit-mm-per-prompt",
            json.dumps({"image": 0, "video": 0, "audio": 0}),
        ]
        if MODEL_REVISION:
            cmd += ["--revision", MODEL_REVISION]
        subprocess.Popen(cmd)
