"""Python-side convenience layer for constructing Hyperion instances.

The helpers in this module mirror the Rust-side `InstanceCreateInfo` structs,
making it straightforward to spin up engines from notebooks or scripts while
still benefiting from type validation.
"""

import hypi._sys as lib # type: ignore
from hypi.api.ext.hylog import LogCreateInfoEXT, LogLevelEXT, LogMessageEXT
from pydantic.dataclasses import dataclass
from pydantic import Field
from enum import StrEnum

class InstanceEXT(StrEnum):
    """Enumeration of built-in extensions that can be enabled on an instance."""

    LOGGER = "__EXT_hyperion_logger"

@dataclass
class Version:
    """Represents a version with major, minor, and patch numbers."""
    major: int
    minor: int
    patch: int

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"
    
    def to_tuple(self) -> tuple[int, int, int]:
        return (self.major, self.minor, self.patch)
    
    @classmethod
    def parse(cls, version_str: str) -> 'Version':
        """Parse a version string in the format 'major.minor.patch'."""
        parts = version_str.split('.')
        if len(parts) != 3:
            raise ValueError("Version string must be in 'major.minor.patch' format")
        major, minor, patch = map(int, parts)
        return cls(major, minor, patch)

@dataclass
class ApplicationInfo:
    """Holds metadata about the application and its host engine."""
    application_name: str
    application_version: Version
    engine_name: str
    engine_version: Version

@dataclass
class InstanceCreateInfo:
    """Aggregates everything Hyperion needs to spin up a new instance."""
    application_info: ApplicationInfo
    enabled_extensions: list[str]
    ext: list[object] = Field(default_factory=list)

def create_instance(create_info: InstanceCreateInfo) -> lib.Instance:
    """Create an instance with the given creation info.

    Parameters
    ----------
    create_info:
        A fully populated :class:`InstanceCreateInfo`. The object is converted
        into the ABI-compatible representation expected by the Rust runtime.

    Returns
    -------
    hypi._sys.Instance
        Handle to the native Hyperion instance. Keep it alive for as long as
        you intend to interact with the core engine.
    """
    assert isinstance(create_info, InstanceCreateInfo), "create_info must be an InstanceCreateInfo"
    return lib._hy_create_instance(create_info)

# Exported names
__all__ = [
    "InstanceCreateInfo",
    "ApplicationInfo",
    "Version",
    "InstanceEXT",
    "LogCreateInfoEXT",
    "LogLevelEXT",
    "LogMessageEXT",
    "create_instance",
]
