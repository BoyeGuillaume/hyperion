use smallvec::SmallVec;

use crate::encoding::{DynBuf, RawEncodable};

pub mod node_ref;

struct WalkerFrame<A> {
    pos_start: u64,
    pos_end: u64,
    delta: i64,
    userdata: A,
}

struct TrackedRegion {
    track_index: u32,
    pos_start: u64,
    pos_end: u64,
}

/// A depth-first walker that edits a binary buffer by inserting/removing bytes.
///
/// This is the naive implementation: insert/remove cause immediate shifts in the
/// underlying buffer. As a result, edits can move bytes of already-closed sibling
/// regions. This keeps the implementation simple and correct, but may be O(n)
/// per edit in the worst case.
pub struct Walker<'a, A> {
    buffer: &'a mut DynBuf,
    stack: SmallVec<[WalkerFrame<A>; 8]>,
    tracked_regions: SmallVec<[TrackedRegion; 3]>, // tracked_regions is sorted by track_index
    next_track_index: u32,
}

pub struct WalkerNode<'a, A> {
    walker: &'a Walker<'a, A>,
}

impl<'a, A> Walker<'a, A> {
    #[inline]
    fn track(&mut self, start: u64, end: u64) -> u32 {
        // We know next_track_index is increasing and there last is always the largest
        let track_index = self.next_track_index;
        self.next_track_index += 1;
        self.tracked_regions.push(TrackedRegion {
            track_index,
            pos_start: start,
            pos_end: end,
        });
        track_index
    }

    #[inline]
    fn get_tracked_index(&self, track_index: u32) -> Option<usize> {
        // Binary search since tracked_regions is sorted by track_index
        self.tracked_regions
            .binary_search_by_key(&track_index, |r| r.track_index)
            .ok()
    }

    #[inline]
    fn get_tracked(&self, track_index: u32) -> Option<&TrackedRegion> {
        self.get_tracked_index(track_index)
            .and_then(|i| self.tracked_regions.get(i))
    }

    #[inline]
    fn reset_tracked(&mut self) {
        self.tracked_regions.clear();
    }

    #[inline]
    fn current_span(&self) -> Option<(u64, u64)> {
        self.stack.last().map(|f| (f.pos_start, f.pos_end))
    }

    fn update_current(&mut self, e: impl RawEncodable) {
        if self.is_root() {
            self.buffer.clear();
            e.encode_dynbuf(&mut self.buffer);
        } else {
            // Retrieve the current soan
            let (start, end) = self.current_span().unwrap();

            // Ensure the span matches the buffer
        }
    }

    /// Create a new walker over the provided dynamic buffer.
    pub fn new(buffer: &'a mut DynBuf) -> Self {
        Self {
            buffer,
            stack: SmallVec::new(),
            tracked_regions: SmallVec::new(),
            next_track_index: 0,
        }
    }

    /// Checks whether the walker is at the root level (no open frames).
    #[inline]
    pub fn is_root(&self) -> bool {
        self.stack.is_empty()
    }

    /// Current buffer length (bytes).
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns the current stack depth. The root walker has depth 0.
    #[inline]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Pop the top frame off the stack, update the parent as needed
    pub(crate) fn pop_frame(&mut self) {}
}
