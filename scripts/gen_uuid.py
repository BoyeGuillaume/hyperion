import uuid
import pyperclip as pclip

new_uuid = str(uuid.uuid4())
pclip.copy(new_uuid)
print(f"[uuid]: {new_uuid} (also copied to clipboard)")