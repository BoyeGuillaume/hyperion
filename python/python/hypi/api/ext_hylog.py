from enum import IntEnum
from typing import Callable
from pydantic.dataclasses import dataclass
from pydantic import Field

class LogLevelEXT(IntEnum):
    TRACE = 0
    DEBUG = 1
    INFO = 2
    WARN = 3
    ERROR = 4

@dataclass
class LogCreateInfoEXT:
    """Holds configuration for the logger extension."""
    level: int
    callback: Callable[[any], None] = Field(default_factory=lambda _x: None)

