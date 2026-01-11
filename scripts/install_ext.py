# Retrieve the path to the extension
import os
from os.path import dirname, abspath, join

# Check if there is an environment variable specifying the extension path
config_path = os.environ.get("HY_CONFIG_PATH")
if config_path is None:
    # If not, use the default path depending on the OS
    if os.name == "nt": # Windows under %APPDATA%/hyperion/meta.toml
        config_path = join(os.environ.get("APPDATA", ""), "hyperion", "meta.toml")
    else: # Unix-like under $XDG_CONFIG_HOME/hyperion/meta.toml or $HOME/.config/hyperion/meta.toml
        xdg_config_home = os.environ.get("XDG_CONFIG_HOME", os.environ.get("HOME", "") + "/.config")
        config_path = join(xdg_config_home, "hyperion", "meta.toml")

# Determine path to the extension directory
root_path = abspath(dirname(dirname(__file__)))
target_dir = join(root_path, "target", "debug")

# Print the determined extension path
print(f"Configuration path: {config_path}")
print(f"Extension target directory: {target_dir}")

# Write the configuration path to a file for later use
configuration_raw = f"""# Auto-generated configuration file
[[ext]]
uuid = "20189b61-7279-46fa-9ba2-5f0360bf5b51"
path = "{join(target_dir, 'libhylog.so')}"
name = "__EXT_hyperion_logger"
"""

# Ensure the directory for the config file exists
with open(config_path, "w") as config_file:
    config_file.write(configuration_raw)

