import hypi._sys as lib # type: ignore
from pydantic.dataclasses import dataclass

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
    
    def parse(version_str: str) -> 'Version':
        """Parse a version string in the format 'major.minor.patch'."""
        parts = version_str.split('.')
        if len(parts) != 3:
            raise ValueError("Version string must be in 'major.minor.patch' format")
        major, minor, patch = map(int, parts)
        return Version(major, minor, patch)

@dataclass
class ApplicationInfo:
    """Holds metadata about the application."""
    application_name: str
    application_version: Version
    engine_name: str
    engine_version: Version

@dataclass
class InstanceCreateInfo:
    """Holds information required to create an instance."""
    application_info: ApplicationInfo
    enabled_extensions: list[str]

def create_instance(create_info: InstanceCreateInfo) -> lib.Instance:
    """Create an instance with the given creation info."""
    assert isinstance(create_info, InstanceCreateInfo), "create_info must be an InstanceCreateInfo"
    return lib._hy_create_instance(create_info)
