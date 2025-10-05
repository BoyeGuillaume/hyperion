use std::marker::PhantomData;

pub struct NodeRef<'a, A> {
    marker: PhantomData<&'a A>,
    track_index: u16,
}
