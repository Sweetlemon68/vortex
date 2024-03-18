use crate::array::primitive::PrimitiveArray;
use crate::array::Array;
use crate::compute::flatten::flatten_bool;
use crate::compute::scalar_at::scalar_at;
use crate::error::VortexResult;
use crate::ptype::NativePType;
use crate::stats::Stat;
use arrow_buffer::{ArrowNativeType, NullBuffer, OffsetBuffer, ScalarBuffer};

pub fn as_scalar_buffer<T: NativePType + ArrowNativeType>(
    array: PrimitiveArray,
) -> ScalarBuffer<T> {
    assert_eq!(array.ptype(), &T::PTYPE);
    ScalarBuffer::from(array.buffer().clone())
}

pub fn as_offset_buffer<T: NativePType + ArrowNativeType>(
    array: PrimitiveArray,
) -> OffsetBuffer<T> {
    OffsetBuffer::new(as_scalar_buffer(array))
}

pub fn as_nulls(validity: Option<&dyn Array>) -> VortexResult<Option<NullBuffer>> {
    if validity.is_none() {
        return Ok(None);
    }

    // Short-circuit if the validity is constant
    let validity = validity.unwrap();
    if validity
        .stats()
        .get_as::<bool>(&Stat::IsConstant)
        .unwrap_or_default()
    {
        if scalar_at(validity, 0)?.try_into().unwrap() {
            return Ok(None);
        } else {
            return Ok(Some(NullBuffer::new_null(validity.len())));
        }
    }

    Ok(Some(NullBuffer::new(
        flatten_bool(validity)?.buffer().clone(),
    )))
}
