"""Top-level helpers for interacting with Hyperion from Python."""

from os.path import join
import os
import sys

import hypi._sys as lib  # type: ignore
import hypi.api as api

__version__ = "0.1.2"

if "HY_LD_PATH" not in os.environ:
    search_roots = [join(path, "target", "release") for path in sys.path]
    os.environ["HY_LD_PATH"] = os.pathsep.join(search_roots)

def factorial(n: int) -> int:
    """Compute the factorial of a non-negative integer n."""
    return lib.factorial(n)

def fibonacci(n: int) -> int:
    """Compute the n-th Fibonacci number."""
    return lib.fibonacci(n)