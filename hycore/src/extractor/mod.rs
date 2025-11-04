use hyinstr::modules::Function;

use crate::attributes::function::FunctionMetadata;

pub struct ExtractorContext<'a> {
    // TODO: Add things such as type information, other functions properties, etc.
    phantom: std::marker::PhantomData<&'a ()>,
}

pub trait PropertiesExtractor {
    fn extract_function(&self, metadata: &mut FunctionMetadata, functions: &Function);
}
