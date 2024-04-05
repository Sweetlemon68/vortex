use arrow_buffer::{NullBuffer, OffsetBuffer, ScalarBuffer};
use vortex_error::VortexResult;

use crate::array::primitive::PrimitiveArray;
use crate::ptype::NativePType;
use crate::validity::Validity;
use crate::view::AsView;

pub fn as_scalar_buffer<T: NativePType>(array: PrimitiveArray) -> ScalarBuffer<T> {
    assert_eq!(array.ptype(), T::PTYPE);
    ScalarBuffer::from(array.buffer().clone())
}

pub fn as_offset_buffer<T: NativePType>(array: PrimitiveArray) -> OffsetBuffer<T> {
    OffsetBuffer::new(as_scalar_buffer(array))
}

pub fn as_nulls(validity: Validity) -> VortexResult<Option<NullBuffer>> {
    match validity {
        Validity::Valid(_) => Ok(None),
        Validity::Invalid(_) => Ok(Some(NullBuffer::new_null(validity.as_view().len()))),
        Validity::Array(_) => Ok(Some(NullBuffer::new(
            validity.to_bool_array().buffer().clone(),
        ))),
    }
}