"""Python bindings for the Hyperion logger extension configuration objects."""

from enum import IntEnum
from typing import Callable, TYPE_CHECKING
from pydantic.dataclasses import dataclass
from pydantic import Field

# if TYPE_CHECKING:
from datetime import datetime


class LogLevelEXT(IntEnum):
    """Mirror of the Rust-side `LogLevelEXT` enumeration."""

    TRACE = 0
    DEBUG = 1
    INFO = 2
    WARN = 3
    ERROR = 4

@dataclass
class LogMessageEXT:
    """Represents a log message emitted by the logger extension."""
    level: LogLevelEXT
    message: str
    timepoint: datetime | None = None
    module: str | None = None
    file: str | None = None
    line: int | None = None
    thread_name: str | None = None

@dataclass
class LogCreateInfoEXT:
    """Holds configuration for the logger extension."""
    level: LogLevelEXT = LogLevelEXT.INFO
    callback: Callable[[LogMessageEXT], None] = Field(default_factory=lambda: lambda _: None)

    def __post_init__(self):
        """Wrap the callback so that native structs turn into typed dataclasses."""

        # Wrap the callback to ensure it matches the actual signature
        original_callback = self.callback
        def wrapped_callback(log_message):
            message = LogMessageEXT(
                level=LogLevelEXT(log_message["level"]),
                message=log_message["message"],
                timepoint=log_message["timepoint"],
                module=log_message["module"],
                file=log_message["file"],
                line=log_message["line"],
                thread_name=log_message["thread_name"],
            )
            original_callback(message)

        self.callback = wrapped_callback

