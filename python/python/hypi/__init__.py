import hypi._sys as lib # type: ignore

def factorial(n: int) -> int:
    """Compute the factorial of a non-negative integer n."""
    return lib.factorial(n)