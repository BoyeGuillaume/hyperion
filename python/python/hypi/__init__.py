import hypi._sys as lib # type: ignore

def factorial(n: int) -> int:
    """Compute the factorial of a non-negative integer n."""
    return lib.factorial(n)

def fibonacci(n: int) -> int:
    """Compute the n-th Fibonacci number."""
    return lib.fibonacci(n)