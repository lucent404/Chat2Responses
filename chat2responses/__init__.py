"""
Chat2Responses Python shim.

Provides a minimal interface to start/stop the relay process.
The actual binary is installed to PATH by the wheel.
"""

import os
import shutil
import subprocess
from pathlib import Path


def _find_binary() -> Path:
    path = shutil.which("chat2responses")
    if path:
        return Path(path)
    # Fallback: look next to this file (editable / dev install)
    local = Path(__file__).parent / "_bin" / "chat2responses"
    if local.exists():
        return local
    raise FileNotFoundError(
        "chat2responses binary not found. "
        "Install with: pip install chat2responses  or  cargo install chat2responses"
    )


def start(
    port: int = 4444,
    database_url: str = "sqlite://data/chat2responses.db",
    secret: str = "",
) -> subprocess.Popen:
    """Start Chat2Responses as a background process and return the Popen handle."""
    env = os.environ.copy()
    if secret:
        env["CHAT2RESPONSES_SECRET"] = secret

    return subprocess.Popen(
        [str(_find_binary()), "--port", str(port), "--database-url", database_url],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )


__all__ = ["start"]
