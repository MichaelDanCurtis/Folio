#!/usr/bin/env python3
"""Fetch the pinned official native Codex package, without requiring Node.js."""
import base64
import hashlib
import io
from pathlib import Path
import ssl
import subprocess
import tarfile
import urllib.error
import urllib.request

URL = 'https://registry.npmjs.org/@openai/codex/-/codex-0.154.0-darwin-arm64.tgz'
SHA512 = 'HP/vJCH/t2hB9Kg6hotN9UglClJ6/z584fal5lEP14C9gNAgAQS4/kTQC7l5V+BA3TqwDPwINSjul28cX8AYXg=='
root = Path(__file__).resolve().parent.parent / 'runtime'
root.mkdir(exist_ok=True)
try:
    with urllib.request.urlopen(URL, timeout=120) as response:
        archive = response.read()
except urllib.error.URLError as error:
    # python.org builds ship without the system trust store; curl uses it.
    # The checksum below still verifies whatever is downloaded.
    if not isinstance(error.reason, ssl.SSLError):
        raise
    archive = subprocess.run(['curl', '-fsSL', '--max-time', '120', URL], check=True, capture_output=True).stdout
if base64.b64encode(hashlib.sha512(archive).digest()).decode() != SHA512:
    raise SystemExit('Codex package checksum mismatch; nothing extracted.')
with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as tar:
    for member in tar.getmembers():
        path = Path(member.name)
        if path.is_absolute() or '..' in path.parts or member.issym() or member.islnk():
            raise SystemExit('Unexpected archive entry; nothing extracted.')
    tar.extractall(root, filter='data') if hasattr(tarfile, 'data_filter') else tar.extractall(root)
print(root / 'package/vendor/aarch64-apple-darwin')
