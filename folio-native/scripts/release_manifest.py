"""Write package identity from the completed ZIP, never from an earlier release."""
import hashlib
import json
import plistlib
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from zipfile import ZipFile

archive = Path(sys.argv[1])
with ZipFile(archive) as bundle:
    info = plistlib.loads(bundle.read('Folio Native.app/Contents/Info.plist'))
    corrupt = bundle.testzip()
    if corrupt:
        raise ValueError(f'Corrupt archive member: {corrupt}')
# Verify the actual distribution archive outside synced-folder metadata, not
# just the app from which it was created. TemporaryDirectory owns this copy.
with tempfile.TemporaryDirectory(prefix='folio-distribution-check-') as temp:
    subprocess.run(['ditto', '-x', '-k', str(archive), temp], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict',
                    str(Path(temp) / 'Folio Native.app')], check=True)
with archive.open('rb') as stream:
    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
manifest = {
    'version': info['CFBundleShortVersionString'],
    'build': info['CFBundleVersion'],
    'bundleId': info['CFBundleIdentifier'],
    'zipBytes': archive.stat().st_size,
    'sha256': digest,
    'zipSignatureVerified': True,
    'generatedAt': datetime.now(timezone.utc).isoformat(),
    'validation': 'See VALIDATION.md for dated and scoped test evidence; package identity is not a parity claim.',
}
archive.with_name('manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(f"Manifest verified for Folio {manifest['version']}")
