// (c) Copyright 2024 Fulcrum Technologies, Inc. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::any::Any;
use std::sync::{Arc, RwLock};

use croaring::{Bitmap, Native};

use compress::roaring_encode;
use vortex::array::{
    check_index_bounds, check_slice_bounds, Array, ArrayKind, ArrayRef, ArrowIterator, Encoding,
    EncodingId, EncodingRef,
};
use vortex::compress::EncodingCompression;
use vortex::dtype::DType;
use vortex::error::{VortexError, VortexResult};
use vortex::formatter::{ArrayDisplay, ArrayFormatter};
use vortex::ptype::PType;
use vortex::scalar::Scalar;
use vortex::serde::{ArraySerde, EncodingSerde};
use vortex::stats::{Stats, StatsSet};

mod compress;
mod serde;
mod stats;

#[derive(Debug, Clone)]
pub struct RoaringIntArray {
    bitmap: Bitmap,
    ptype: PType,
    stats: Arc<RwLock<StatsSet>>,
}

impl RoaringIntArray {
    pub fn new(bitmap: Bitmap, ptype: PType) -> Self {
        Self::try_new(bitmap, ptype).unwrap()
    }

    pub fn try_new(bitmap: Bitmap, ptype: PType) -> VortexResult<Self> {
        if !ptype.is_unsigned_int() {
            return Err(VortexError::InvalidPType(ptype));
        }

        Ok(Self {
            bitmap,
            ptype,
            stats: Arc::new(RwLock::new(StatsSet::new())),
        })
    }

    pub fn bitmap(&self) -> &Bitmap {
        &self.bitmap
    }

    pub fn ptype(&self) -> PType {
        self.ptype
    }

    pub fn encode(array: &dyn Array) -> VortexResult<Self> {
        match ArrayKind::from(array) {
            ArrayKind::Primitive(p) => Ok(roaring_encode(p)),
            _ => Err(VortexError::InvalidEncoding(array.encoding().id().clone())),
        }
    }
}

impl Array for RoaringIntArray {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn boxed(self) -> ArrayRef {
        Box::new(self)
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn len(&self) -> usize {
        self.bitmap.cardinality() as usize
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.bitmap().is_empty()
    }

    #[inline]
    fn dtype(&self) -> &DType {
        self.ptype.into()
    }

    fn stats(&self) -> Stats {
        Stats::new(&self.stats, self)
    }

    fn scalar_at(&self, index: usize) -> VortexResult<Box<dyn Scalar>> {
        check_index_bounds(self, index)?;
        // Unwrap since we know the index is valid
        let bitmap_value = self.bitmap.select(index as u32).unwrap();
        let scalar: Box<dyn Scalar> = match self.ptype {
            PType::U8 => (bitmap_value as u8).into(),
            PType::U16 => (bitmap_value as u16).into(),
            PType::U32 => bitmap_value.into(),
            PType::U64 => (bitmap_value as u64).into(),
            _ => unreachable!("RoaringIntArray constructor should have disallowed this type"),
        };
        Ok(scalar)
    }

    fn iter_arrow(&self) -> Box<ArrowIterator> {
        todo!()
    }

    fn slice(&self, start: usize, stop: usize) -> VortexResult<ArrayRef> {
        check_slice_bounds(self, start, stop)?;
        todo!()
    }

    #[inline]
    fn encoding(&self) -> EncodingRef {
        &RoaringIntEncoding
    }

    #[inline]
    fn nbytes(&self) -> usize {
        self.bitmap.get_serialized_size_in_bytes::<Native>()
    }

    fn serde(&self) -> &dyn ArraySerde {
        self
    }
}

impl<'arr> AsRef<(dyn Array + 'arr)> for RoaringIntArray {
    fn as_ref(&self) -> &(dyn Array + 'arr) {
        self
    }
}

impl ArrayDisplay for RoaringIntArray {
    fn fmt(&self, f: &mut ArrayFormatter) -> std::fmt::Result {
        f.indent(|indent| indent.writeln(format!("{:?}", self.bitmap())))
    }
}

#[derive(Debug)]
pub struct RoaringIntEncoding;

impl RoaringIntEncoding {
    pub const ID: EncodingId = EncodingId::new("roaring.int");
}

impl Encoding for RoaringIntEncoding {
    fn id(&self) -> &EncodingId {
        &Self::ID
    }

    fn compression(&self) -> Option<&dyn EncodingCompression> {
        Some(self)
    }

    fn serde(&self) -> Option<&dyn EncodingSerde> {
        Some(self)
    }
}

#[cfg(test)]
mod test {
    use vortex::array::primitive::PrimitiveArray;
    use vortex::array::Array;
    use vortex::error::VortexResult;

    use crate::RoaringIntArray;

    #[test]
    pub fn scalar_at() -> VortexResult<()> {
        let ints: &dyn Array = &PrimitiveArray::from_vec::<u32>(vec![2, 12, 22, 32]);
        let array = RoaringIntArray::encode(ints)?;

        assert_eq!(array.scalar_at(0), Ok(2u32.into()));
        assert_eq!(array.scalar_at(1), Ok(12u32.into()));

        Ok(())
    }
}
