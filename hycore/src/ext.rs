use crate::HyResult;
use downcast_rs::{Downcast, impl_downcast};
use std::fmt::Debug;

#[cfg(feature = "pyo3")]
use pyo3::{prelude::*, types::PyList};

/// Empty trait to mark objects that are extension objects, that is portion of the API that is fully extendable
pub trait ExtObject: Downcast + Debug {}
impl_downcast!(ExtObject);

pub type DynExtObject = Box<dyn ExtObject>;

/// List of ext objects that can be passed to the instance create info
#[derive(Debug, Default)]
pub struct ExtList(Vec<DynExtObject>);

impl Into<ExtList> for DynExtObject {
    fn into(self) -> ExtList {
        let mut list = ExtList::new();
        list.push(self)
            .expect("Failed to push ext object to ExtList");
        list
    }
}

macro_rules! recurse_tuple_def_inner {
    (
        $list:expr, $self:expr =>
        $head:tt
        $(, $tail:tt)*
    ) => {
        $list.push(Box::new($self.$head) as DynExtObject)
            .expect("Failed to push ext object to ExtList");
        recurse_tuple_def_inner!($list, $self => $($tail),*);
    };
    ($list:expr, $self:expr => ) => {};
}

macro_rules! recurse_tuple_def {
    ($head:tt) => {};
    (
        $head:tt
        $(, $tail:tt)*
    ) => {
        paste::paste! {
            impl<$( [< _ $tail >]: ExtObject + 'static ),*> Into<ExtList> for ( $( [< _ $tail >] ),* , ) {
                fn into(self) -> ExtList {
                    let mut list = ExtList::new();
                    recurse_tuple_def_inner!(list, self => $($tail),*);
                    list
                }
            }
        }

        recurse_tuple_def!($($tail),*);
    };
}

// Generate definition from 0 to 12
recurse_tuple_def!(13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0);

impl ExtList {
    #[inline]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[inline]
    pub fn push(&mut self, ext: DynExtObject) -> HyResult<()> {
        // Verify that the type of the ext object is not already in the list, to avoid duplicates and ambiguity when retrieving the ext object later
        if self
            .0
            .iter()
            .any(|e| e.as_ref().type_id() == ext.as_ref().type_id())
        {
            return Err(anyhow::anyhow!(
                "Duplicate ext object of type {:?} found in ExtList. All ext objects must be unique.",
                ext.as_ref().type_id()
            ));
        }

        self.0.push(ext);
        Ok(())
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &DynExtObject> {
        self.0.iter()
    }

    #[inline]
    pub fn get<T: ExtObject + 'static>(&self) -> Option<&T> {
        self.0
            .iter()
            .find(|e| e.as_ref().type_id() == std::any::TypeId::of::<T>())
            .and_then(|e| e.as_ref().downcast_ref::<T>())
    }

    #[inline]
    pub fn pop<T: ExtObject + 'static>(&mut self) -> Option<T> {
        if let Some(pos) = self
            .0
            .iter()
            .position(|e| e.as_ref().type_id() == std::any::TypeId::of::<T>())
        {
            let ext = self.0.remove(pos);
            ext.downcast::<T>().ok().map(|boxed| *boxed)
        } else {
            None
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    pub fn contains<T: ExtObject + 'static>(&self) -> bool {
        self.0
            .iter()
            .any(|e| e.as_ref().type_id() == std::any::TypeId::of::<T>())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(feature = "pyo3")]
impl<'a, 'py> FromPyObject<'a, 'py> for ExtList {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let list: Borrowed<PyList> = obj.cast::<PyList>()?;
        let mut ext_list = ExtList::new();

        for item in list.iter() {
            let ext_object: DynExtObject = item.extract()?;
            ext_list.push(ext_object).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to push ext object to ExtList: {}",
                    e
                ))
            })?;
        }

        Ok(ext_list)
    }
}

#[cfg(feature = "cffi")]
impl ExtList {
    /// Get an extension object from an opaque pointer, by iterating over the list and trying to downcast each object to the desired type, and returning the first one that matches
    ///
    /// SAFETY:
    /// - Caller must ensure that the pointer is valid and points to an object of the correct type for the given sType.
    /// - Caller must ensure that the sType correspond to the real type of the object pointed to by ptr.
    #[inline]
    pub unsafe fn from_cffi(mut ptr: *mut std::ffi::c_void) -> HyResult<Self> {
        let mut list = ExtList::new();

        while !ptr.is_null() {
            let (ext_object, next) = unsafe { ExtObjectCFFIInventory::get(ptr)? };
            list.push(ext_object)?;
            ptr = next;
        }

        Ok(list)
    }
}

/// CFFI struct for extension objects, which contains a callback function to create the object from a pointer
#[cfg(feature = "cffi")]
pub struct ExtObjectCFFIInventory {
    pub stype: u32,
    pub callback:
        unsafe fn(ptr: *mut std::ffi::c_void) -> HyResult<(DynExtObject, *mut std::ffi::c_void)>,
}
#[cfg(feature = "cffi")]
inventory::collect!(ExtObjectCFFIInventory);

#[cfg(feature = "cffi")]
impl ExtObjectCFFIInventory {
    /// Get an extension object from an opaque pointer
    ///
    /// SAFETY:
    /// - Caller must ensure that the pointer is valid and points to an object of the correct type for the given sType.
    /// - Caller must ensure that the sType correspond to the real type of the object pointed to by ptr.
    ///
    #[inline]
    pub unsafe fn get(
        ptr: *mut std::ffi::c_void,
    ) -> HyResult<(DynExtObject, *mut std::ffi::c_void)> {
        assert!(!ptr.is_null(), "Pointer must not be null");

        // Retrieve the sType from the pointer, which is always the first field of the struct (u32)
        let stype = unsafe { *(ptr as *const u32) };
        let element = inventory::iter::<Self>
            .into_iter()
            .find(|entry| entry.stype == stype)
            .ok_or_else(|| {
                anyhow::anyhow!("ExtObjectCFFIInventory not found for sType {:?}", stype)
            })?;

        // Call the callback function to get the extension object
        unsafe { (element.callback)(ptr) }
    }

    /// Verify that all stypes in the inventory are unique, and if not, return an error with the duplicate stypes and their callbacks
    #[inline]
    pub fn verify() -> HyResult<()> {
        // Verify that all stypes in the inventory are unique
        let mut seen = std::collections::BTreeSet::new();
        for entry in inventory::iter::<Self> {
            if !seen.insert(entry.stype) {
                return Err(anyhow::anyhow!(
                    "Duplicate stype {:?} found in ExtObjectCFFIInventory. All stypes must be unique. This is likely a bug in the code, please report it to the developers. The duplicate stype was found in the following entries: {}",
                    entry.stype,
                    inventory::iter::<Self>()
                        .into_iter()
                        .filter(|e| e.stype == entry.stype)
                        .map(|e| format!("{:?} (callback: {:p})", e.stype, e.callback as *const ()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        Ok(())
    }
}

/// Python trait for extension objects, which contains a callback function to create the object from a pointer
#[cfg(feature = "pyo3")]
impl<'a, 'py> FromPyObject<'a, 'py> for DynExtObject {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        // Calling this each time is a bit expensive, TODO: lazy assertion or something
        ExtObjectPyInventory::verify().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "ExtObjectPyInventory verification failed: {}",
                e
            ))
        })?;

        // Retrieve the qualified name of the type of the object
        let qualname = obj.get_type().qualname()?;
        let typename = qualname.to_str()?;

        // Find corresponding callback in inventory and call it
        for entry in inventory::iter::<ExtObjectPyInventory> {
            if entry.qualname == typename {
                return Ok((entry.callback)(obj)?);
            }
        }

        // If no callback is found, return an error
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "No extension object found for type '{}'. Possible types are: {}",
            typename,
            inventory::iter::<ExtObjectPyInventory>()
                .map(|entry| entry.qualname)
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

/// Python struct for extension objects, which contains a callback function to create the object from a pointer
#[cfg(feature = "pyo3")]
pub struct ExtObjectPyInventory {
    pub qualname: &'static str,
    pub callback: for<'a, 'py> fn(ptr: Borrowed<'a, 'py, PyAny>) -> PyResult<DynExtObject>,
}

#[cfg(feature = "pyo3")]
impl ExtObjectPyInventory {
    #[inline]
    pub fn verify() -> anyhow::Result<()> {
        // Verify that all qualnames in the inventory are unique
        let mut seen = std::collections::HashSet::new();
        for entry in inventory::iter::<Self> {
            if !seen.insert(entry.qualname) {
                return Err(anyhow::anyhow!(
                    "Duplicate qualname '{}' found in ExtObjectPyInventory. All qualnames must be unique. This is likely a bug in the code, please report it to the developers. The duplicate qualname was found in the following entries: {}",
                    entry.qualname,
                    inventory::iter::<Self>()
                        .into_iter()
                        .filter(|e| e.qualname == entry.qualname)
                        .map(|e| format!(
                            "{} (callback: {:p})",
                            e.qualname, e.callback as *const ()
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        Ok(())
    }
}

#[cfg(feature = "pyo3")]
inventory::collect!(ExtObjectPyInventory);
