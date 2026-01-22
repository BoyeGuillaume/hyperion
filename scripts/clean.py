# Retrieve the path to the extension
import os
import subprocess
import shutil
from os.path import dirname, abspath, join

PARENT_DIR = dirname(dirname(abspath(__file__)))

# Check if there is an environment variable specifying the extension path
hy_home = os.environ.get("HY_HOME")
if hy_home is None:
    # If not, use the default path depending on the OS
    if os.name == "nt": # Windows under %APPDATA%/hyperion/meta.toml
        hy_home = join(os.environ.get("APPDATA", ""), "hyperion")
    else: # Unix-like under $XDG_CONFIG_HOME/hyperion/meta.toml or $HOME/.config/hyperion/meta.toml
        home = os.environ.get("HOME", "")
        hy_home = join(home, ".cache", "hyperion")

# Remove the directory if it exists
if os.path.exists(hy_home):
    import shutil
    shutil.rmtree(hy_home)
    print(f"Removed directory: {hy_home}")
else:
    print(f"Directory does not exist: {hy_home}")

# Also run cargo clean command in PARENT_DIR
subprocess.run(["cargo", "clean"], cwd=PARENT_DIR)
print(f"Ran 'cargo clean' in: {PARENT_DIR}")

# Also run make clean command in cffi/examples
examples_dir = join(PARENT_DIR, "cffi", "examples")
subprocess.run(["make", "clean"], cwd=examples_dir)
print(f"Ran 'make clean' in: {examples_dir}")

# Additionally, remove all .so, .dll, and .dylib files and __pycache__ directories in the python directory (recursively)
python_dir = join(PARENT_DIR, "python", "python")
for root, dirs, files in os.walk(python_dir):
    for file in files:
        if file.endswith((".so", ".dll", ".dylib")):
            file_path = join(root, file)
            os.remove(file_path)
            print(f"Removed file: {file_path}")
    if "__pycache__" in dirs:
        pycache_path = join(root, "__pycache__")
        shutil.rmtree(pycache_path)
        print(f"Removed directory: {pycache_path}")