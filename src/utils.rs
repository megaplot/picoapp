use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Wrapper to pass around the callback.
///
/// Note that originally I was passing around the callback in the form of a Py<PyFunction>>.
/// It looks like PyFunction is only usable for regular functions and lambdas, but not for
/// methods, and therefore also not for `__call__`. This type is less restrictive, because
/// it only takes a PyAny and checks whether it is callable, which seems to be for methods
/// as well. In the end, all we care about is whether it is callable anyway, but note that
/// we are not checking the signature.
#[derive(Debug)]
pub struct Callback(Py<PyAny>);

impl Callback {
    pub fn clone_ref(&self, py: Python<'_>) -> Callback {
        Callback(self.0.clone_ref(py))
    }
    /// Abstraction for our "unary" call of the callback.
    pub fn call(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.0.call_bound(py, (), None)
    }
}

impl<'py> FromPyObject<'py> for Callback {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if obj.is_callable() {
            Ok(Callback(obj.clone().unbind()))
        } else {
            Err(PyValueError::new_err(format!(
                "Invalid callback type (not callable): {:?}",
                obj.get_type().name()?
            )))
        }
    }
}
