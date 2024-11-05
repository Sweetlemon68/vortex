//! Vortex crate containing core logic for encoding and memory representation of [arrays](Array).
//!
//! At the heart of Vortex are [arrays](Array) and [encodings](crate::encoding::ArrayEncoding).
//! Arrays are typed views of memory buffers that hold [scalars](vortex_scalar::Scalar). These
//! buffers can be held in a number of physical encodings to perform lightweight compression that
//! exploits the particular data distribution of the array's values.
//!
//! Every data type recognized by Vortex also has a canonical physical encoding format, which
//! arrays can be [canonicalized](Canonical) into for ease of access in compute functions.
//!

use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::future::ready;

pub use canonical::*;
pub use context::*;
pub use data::*;
pub use implementation::*;
use itertools::Itertools;
pub use metadata::*;
pub use paste;
use stats::Statistics;
pub use typed::*;
pub use view::*;
use vortex_buffer::Buffer;
use vortex_dtype::DType;
use vortex_error::{vortex_err, vortex_panic, VortexExpect, VortexResult};

use crate::array::visitor::{AcceptArrayVisitor, ArrayVisitor};
use crate::compute::ArrayCompute;
use crate::encoding::{ArrayEncodingRef, EncodingId, EncodingRef};
use crate::iter::{ArrayIterator, ArrayIteratorAdapter};
use crate::stats::{ArrayStatistics, ArrayStatisticsCompute};
use crate::stream::{ArrayStream, ArrayStreamAdapter};
use crate::validity::ArrayValidity;
use crate::variants::ArrayVariants;

pub mod accessor;
pub mod aliases;
pub mod array;
pub mod arrow;
mod canonical;
pub mod compress;
pub mod compute;
mod context;
mod data;
pub mod elementwise;
pub mod encoding;
mod implementation;
pub mod iter;
mod metadata;
pub mod stats;
pub mod stream;
mod tree;
mod typed;
pub mod validity;
pub mod variants;
mod view;

pub mod flatbuffers {
    //! Re-exported autogenerated code from the core Vortex flatbuffer definitions.
    pub use vortex_flatbuffers::array::*;
}

/// A central type for all Vortex arrays, which are known length sequences of typed and possibly compressed data.
///
/// This is the main entrypoint for working with in-memory Vortex data, and dispatches work over the underlying encoding or memory representations.
#[derive(Debug, Clone)]
pub struct Array(pub(crate) Inner);

#[derive(Debug, Clone)]
pub(crate) enum Inner {
    /// Owned [`Array`] with serialized metadata, backed by heap-allocated memory.
    Data(ArrayData),
    /// Zero-copy view over flatbuffer-encoded [`Array`] data, created without eager serialization.
    View(ArrayView),
}

impl Array {
    pub fn encoding(&self) -> EncodingRef {
        match &self.0 {
            Inner::Data(d) => d.encoding(),
            Inner::View(v) => v.encoding(),
        }
    }

    /// Returns the number of logical elements in the array.
    #[allow(clippy::same_name_method)]
    pub fn len(&self) -> usize {
        match &self.0 {
            Inner::Data(d) => d.len(),
            Inner::View(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.0 {
            Inner::Data(d) => d.is_empty(),
            Inner::View(v) => v.is_empty(),
        }
    }

    /// Total size of the array in bytes, including all children and buffers.
    pub fn nbytes(&self) -> usize {
        self.with_dyn(|a| a.nbytes())
    }

    pub fn child<'a>(&'a self, idx: usize, dtype: &'a DType, len: usize) -> VortexResult<Self> {
        match &self.0 {
            Inner::Data(d) => d.child(idx, dtype, len).cloned(),
            Inner::View(v) => v
                .child(idx, dtype, len)
                .map(|view| Array(Inner::View(view))),
        }
    }

    /// Returns a Vec of Arrays with all of the array's child arrays.
    pub fn children(&self) -> Vec<Array> {
        match &self.0 {
            Inner::Data(d) => d.children().iter().cloned().collect_vec(),
            Inner::View(v) => v.children(),
        }
    }

    /// Returns the number of child arrays
    pub fn nchildren(&self) -> usize {
        match &self.0 {
            Inner::Data(d) => d.nchildren(),
            Inner::View(v) => v.nchildren(),
        }
    }

    pub fn depth_first_traversal(&self) -> ArrayChildrenIterator {
        ArrayChildrenIterator::new(self.clone())
    }

    /// Count the number of cumulative buffers encoded by self.
    pub fn cumulative_nbuffers(&self) -> usize {
        self.children()
            .iter()
            .map(|child| child.cumulative_nbuffers())
            .sum::<usize>()
            + if self.buffer().is_some() { 1 } else { 0 }
    }

    /// Return the buffer offsets and the total length of all buffers, assuming the given alignment.
    /// This includes all child buffers.
    pub fn all_buffer_offsets(&self, alignment: usize) -> Vec<u64> {
        let mut offsets = vec![];
        let mut offset = 0;

        for col_data in self.depth_first_traversal() {
            if let Some(buffer) = col_data.buffer() {
                offsets.push(offset as u64);

                let buffer_size = buffer.len();
                let aligned_size = (buffer_size + (alignment - 1)) & !(alignment - 1);
                offset += aligned_size;
            }
        }
        offsets.push(offset as u64);

        offsets
    }

    /// Get back the (possibly owned) metadata for the array.
    ///
    /// View arrays will return a reference to their bytes, while heap-backed arrays
    /// must first serialize their metadata, returning an owned byte array to the caller.
    pub fn metadata(&self) -> VortexResult<Cow<[u8]>> {
        match &self.0 {
            Inner::Data(array_data) => {
                // Heap-backed arrays must first try and serialize the metadata.
                let owned_meta: Vec<u8> = array_data
                    .metadata()
                    .try_serialize_metadata()?
                    .as_ref()
                    .to_owned();

                Ok(Cow::Owned(owned_meta))
            }
            Inner::View(array_view) => {
                // View arrays have direct access to metadata bytes.
                array_view
                    .metadata()
                    .ok_or_else(|| vortex_err!("things"))
                    .map(Cow::Borrowed)
            }
        }
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        match &self.0 {
            Inner::Data(d) => d.buffer(),
            Inner::View(v) => v.buffer(),
        }
    }

    pub fn into_buffer(self) -> Option<Buffer> {
        match self.0 {
            Inner::Data(d) => d.into_buffer(),
            Inner::View(v) => v.buffer().cloned(),
        }
    }

    pub fn into_array_iterator(self) -> impl ArrayIterator {
        ArrayIteratorAdapter::new(self.dtype().clone(), std::iter::once(Ok(self)))
    }

    pub fn into_array_stream(self) -> impl ArrayStream {
        ArrayStreamAdapter::new(
            self.dtype().clone(),
            futures_util::stream::once(ready(Ok(self))),
        )
    }

    /// Checks whether array is of a given encoding.
    pub fn is_encoding(&self, id: EncodingId) -> bool {
        self.encoding().id() == id
    }

    #[inline]
    pub fn with_dyn<R, F>(&self, mut f: F) -> R
    where
        F: FnMut(&dyn ArrayTrait) -> R,
    {
        let mut result = None;

        self.encoding()
            .with_dyn(self, &mut |array| {
                // Sanity check that the encoding implements the correct array trait
                debug_assert!(
                    match array.dtype() {
                        DType::Null => array.as_null_array().is_some(),
                        DType::Bool(_) => array.as_bool_array().is_some(),
                        DType::Primitive(..) => array.as_primitive_array().is_some(),
                        DType::Utf8(_) => array.as_utf8_array().is_some(),
                        DType::Binary(_) => array.as_binary_array().is_some(),
                        DType::Struct(..) => array.as_struct_array().is_some(),
                        DType::List(..) => array.as_list_array().is_some(),
                        DType::Extension(..) => array.as_extension_array().is_some(),
                    },
                    "Encoding {} does not implement the variant trait for {}",
                    self.encoding().id(),
                    array.dtype()
                );

                result = Some(f(array));
                Ok(())
            })
            .unwrap_or_else(|err| {
                vortex_panic!(
                    err,
                    "Failed to convert Array to {}",
                    std::any::type_name::<dyn ArrayTrait>()
                )
            });

        // Now we unwrap the optional, which we know to be populated by the closure.
        result.vortex_expect("Failed to get result from Array::with_dyn")
    }
}

/// A depth-first pre-order iterator over a ArrayData.
pub struct ArrayChildrenIterator {
    stack: Vec<Array>,
}

impl ArrayChildrenIterator {
    pub fn new(array: Array) -> Self {
        Self { stack: vec![array] }
    }
}

impl Iterator for ArrayChildrenIterator {
    type Item = Array;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.stack.pop()?;
        for child in next.children().into_iter().rev() {
            self.stack.push(child);
        }
        Some(next)
    }
}

pub trait ToArray {
    fn to_array(&self) -> Array;
}

/// Consume `self` and turn it into an [`Array`] infallibly.
///
/// Implementation of this array should never fail.
pub trait IntoArray {
    fn into_array(self) -> Array;
}

pub trait ToArrayData {
    fn to_array_data(&self) -> ArrayData;
}

/// Collects together the behavior of an array.
pub trait ArrayTrait:
    ArrayEncodingRef
    + ArrayCompute
    + ArrayDType
    + ArrayLen
    + ArrayVariants
    + IntoCanonical
    + ArrayValidity
    + AcceptArrayVisitor
    + ArrayStatistics
    + ArrayStatisticsCompute
    + ToArrayData
{
    /// Total size of the array in bytes, including all children and buffers.
    fn nbytes(&self) -> usize {
        let mut visitor = NBytesVisitor(0);
        self.accept(&mut visitor)
            .vortex_expect("Failed to get nbytes from Array");
        visitor.0
    }
}

pub trait ArrayDType {
    // TODO(ngates): move into ArrayTrait?
    fn dtype(&self) -> &DType;
}

impl<T: AsRef<Array>> ArrayDType for T {
    fn dtype(&self) -> &DType {
        match &self.as_ref().0 {
            Inner::Data(array_data) => array_data.dtype(),
            Inner::View(array_view) => array_view.dtype(),
        }
    }
}

pub trait ArrayLen {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool;
}

impl<T: AsRef<Array>> ArrayLen for T {
    fn len(&self) -> usize {
        match &self.as_ref().0 {
            Inner::Data(d) => d.len(),
            Inner::View(v) => v.len(),
        }
    }

    fn is_empty(&self) -> bool {
        match &self.as_ref().0 {
            Inner::Data(d) => d.is_empty(),
            Inner::View(v) => v.is_empty(),
        }
    }
}

struct NBytesVisitor(usize);

impl ArrayVisitor for NBytesVisitor {
    fn visit_child(&mut self, _name: &str, array: &Array) -> VortexResult<()> {
        self.0 += array.with_dyn(|a| a.nbytes());
        Ok(())
    }

    fn visit_buffer(&mut self, buffer: &Buffer) -> VortexResult<()> {
        self.0 += buffer.len();
        Ok(())
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let prefix = match &self.0 {
            Inner::Data(_) => "",
            Inner::View(_) => "$",
        };
        write!(
            f,
            "{}{}({}, len={})",
            prefix,
            self.encoding().id(),
            self.dtype(),
            self.len()
        )
    }
}

impl ToArrayData for Array {
    fn to_array_data(&self) -> ArrayData {
        match &self.0 {
            Inner::Data(d) => d.clone(),
            Inner::View(_) => self.with_dyn(|a| a.to_array_data()),
        }
    }
}

impl ToArray for ArrayData {
    fn to_array(&self) -> Array {
        Array(Inner::Data(self.clone()))
    }
}

impl ToArray for ArrayView {
    fn to_array(&self) -> Array {
        Array(Inner::View(self.clone()))
    }
}

impl IntoArray for ArrayView {
    fn into_array(self) -> Array {
        Array(Inner::View(self))
    }
}

impl From<Array> for ArrayData {
    fn from(value: Array) -> ArrayData {
        match value.0 {
            Inner::Data(d) => d,
            Inner::View(_) => value.with_dyn(|v| v.to_array_data()),
        }
    }
}

impl From<ArrayData> for Array {
    fn from(value: ArrayData) -> Array {
        Array(Inner::Data(value))
    }
}

impl<T: AsRef<Array>> ArrayStatistics for T {
    fn statistics(&self) -> &(dyn Statistics + '_) {
        match &self.as_ref().0 {
            Inner::Data(d) => d.statistics(),
            Inner::View(v) => v.statistics(),
        }
    }
}
