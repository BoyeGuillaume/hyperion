import argparse
import re
from os.path import join, dirname, abspath

WORKSPACE_ROOT = abspath(dirname(dirname(__file__)))

def main():
    parser = argparse.ArgumentParser(description="Bump the version number.")
    parser.add_argument("version", type=str, help="New version number (e.g., 0.1.2)")
    args = parser.parse_args()

    version = args.version.strip()
    version_pattern = r"^\d+\.\d+\.\d+$"
    if not re.match(version_pattern, version):
        print(f"Error: Version '{version}' is not in the format X.Y.Z")
        return

    # Update the __version__ in hypi/__init__.py
    init_file = join(WORKSPACE_ROOT, "python", "python", "hypi", "__init__.py")
    with open(init_file, "r") as f:
        lines = f.readlines()
    with open(init_file, "w") as f:
        for line in lines:
            if line.startswith("__version__ ="):
                f.write(f'__version__ = "{version}"\n')
            else:
                f.write(line)
    
    # Update the version in pyproject.toml
    pyproject_file = join(WORKSPACE_ROOT, "python", "pyproject.toml")
    with open(pyproject_file, "r") as f:
        lines = f.readlines()
    with open(pyproject_file, "w") as f:
        for line in lines:
            if line.strip().startswith('version ='):
                f.write(f'version = "{version}"\n')
            else:
                f.write(line)

    # Similarly for hyinstr/Cargo.toml, hycore/Cargo.toml, cffi/Cargo.toml
    for crate in ["hyinstr", "hycore", "cffi"]:
        cargo_file = join(WORKSPACE_ROOT, crate, "Cargo.toml")
        with open(cargo_file, "r") as f:
            lines = f.readlines()
        with open(cargo_file, "w") as f:
            for line in lines:
                if line.strip().startswith('version ='):
                    f.write(f'version = "{version}"\n')
                else:
                    f.write(line)

if __name__ == "__main__":
    main()
