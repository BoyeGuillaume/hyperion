import hypi._sys as lib # type: ignore
import hypi.api as api
from os.path import join
import sys
import os

if not "HY_LD_PATH" in os.environ:
    os.environ["HY_LD_PATH"] = os.path.pathsep.join([join(path, "target", "release") for path in sys.path])


__version__ = "0.1.1"

def factorial(n: int) -> int:
    """Compute the factorial of a non-negative integer n."""
    return lib.factorial(n)

def fibonacci(n: int) -> int:
    """Compute the n-th Fibonacci number."""
    return lib.fibonacci(n)