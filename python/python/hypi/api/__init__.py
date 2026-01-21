"""Python-side convenience layer for constructing Hyperion instances.

The helpers in this module mirror the Rust-side `InstanceCreateInfo` structs,
making it straightforward to spin up engines from notebooks or scripts while
still benefiting from type validation.
"""

import hypi._sys as lib # type: ignore
from hypi.api.ext.hylog import LogCreateInfoEXT, LogLevelEXT, LogMessageEXT
from pydantic.dataclasses import dataclass
from pydantic import Field
from typing import Optional
from enum import StrEnum, IntEnum

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
    node_id: int = 0
    ext: list[object] = Field(default_factory=list)

    def __post_init__(self):
        # Ensure node_id does not exceed u32 limits
        if not (0 <= self.node_id <= 0xFFFFFFFF):
            raise ValueError("node_id must be between 0 and 2^32 - 1")

class ModuleSourceType(IntEnum):
    """Enumeration of source module types."""
    ASSEMBLY = 0

@dataclass
class ModuleSourceInfo:
    """Information about a source module to be loaded into the instance."""
    source_type: ModuleSourceType
    data: str
    filename: Optional[str] = None

@dataclass
class ModuleCompileInfo:
    """Information about how to compile a source module."""
    sources: list[ModuleSourceInfo]

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

def compile_module(instance: lib.Instance, compile_info: ModuleCompileInfo) -> bytes:
    """Compile a source module within the context of the given instance.

    Parameters
    ----------
    instance:
        Handle to a running Hyperion instance.
    compile_info:
        A fully populated :class:`ModuleCompileInfo` describing the source
        module to be compiled.

    Returns
    ------- 
    bytes
        The compiled module as a byte array.
    """
    assert isinstance(instance, lib.Instance), "instance must be a lib.Instance"
    assert isinstance(compile_info, ModuleCompileInfo), "compile_info must be a ModuleCompileInfo"
    return lib._hy_compile_module(instance, compile_info)

def load_module(instance: lib.Instance, module_data: bytes) -> lib.Module:
    """Load a compiled module into the given instance.

    Parameters
    ----------
    instance:
        Handle to a running Hyperion instance.
    module_data:
        The compiled module as a byte array.

    Returns
    -------
    hypi._sys.Module
        Handle to the loaded module within the instance.
    """
    assert isinstance(instance, lib.Instance), "instance must be a lib.Instance"
    assert isinstance(module_data, bytes), "module_data must be bytes"
    return lib._hy_load_module(instance, module_data)

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
