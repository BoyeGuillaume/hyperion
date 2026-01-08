use pyo3::prelude::*;

/// Computes the factorial of a number.
#[pyfunction]
fn factorial(n: u64) -> PyResult<u64> {
    let mut res = 1u64;
    for i in 2..=n {
        res = res.checked_mul(i).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyOverflowError, _>("integer overflow")
        })?;
    }
    Ok(res)
}

#[pymodule]
#[pyo3(name = "_sys")]
fn hypi_sys(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(factorial, m)?)?;
    Ok(())
}

// /// A Python module implemented in Rust.
// #[pymodule]
// mod hypi_sys {
//     use pyo3::prelude::*;

//     /// Formats the sum of two numbers as string.
//     #[pyfunction]
//     fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
//         Ok((a + b).to_string())
//     }
// }
