use std::iter;
use std::sync::{Arc, RwLock};

use arrow::array::cast::AsArray;
use arrow::array::types::UInt8Type;
use arrow::array::{ArrayRef, StringBuilder};

use crate::array::stats::{Stats, StatsSet};
use crate::array::{Array, ArrayEncoding, ArrowIterator};
use crate::arrow::CombineChunks;
use crate::error::EncResult;
use crate::scalar::{BinaryScalar, Scalar, Utf8Scalar};
use crate::types::{DType, IntWidth, Signedness};

mod stats;

#[derive(Clone, Copy)]
#[repr(C, align(8))]
struct Inlined {
    size: u32,
    data: [u8; 12],
}

impl Inlined {
    #[allow(dead_code)]
    pub fn new(value: &str) -> Self {
        assert!(
            value.len() < 13,
            "Inlined strings must be shorter than 13 characters, {} given",
            value.len()
        );
        let mut inlined = Inlined {
            size: value.len() as u32,
            data: [0u8; 12],
        };
        inlined.data[..value.len()].copy_from_slice(value.as_bytes());
        inlined
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(8))]
struct Ref {
    size: u32,
    prefix: [u8; 4],
    buffer_index: u32,
    offset: u32,
}

#[derive(Clone, Copy)]
#[repr(C, align(8))]
union BinaryView {
    inlined: Inlined,
    _ref: Ref,
}

impl BinaryView {
    #[inline]
    pub fn from_le_bytes(bytes: &[u8]) -> BinaryView {
        let size = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if size > 12 {
            BinaryView {
                _ref: Ref {
                    size,
                    prefix: bytes[4..8].try_into().unwrap(),
                    buffer_index: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
                    offset: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                },
            }
        } else {
            BinaryView {
                inlined: Inlined {
                    size,
                    data: bytes[4..16].try_into().unwrap(),
                },
            }
        }
    }

    #[inline]
    #[allow(clippy::wrong_self_convention)]
    #[allow(dead_code)]
    pub fn to_le_bytes(&self) -> [u8; 16] {
        let mut bytes: [u8; 16] = [0; 16];
        unsafe {
            bytes[0..4].copy_from_slice(&self.inlined.size.to_le_bytes());
            if self.inlined.size > 12 {
                bytes[4..8].copy_from_slice(&self._ref.prefix);
                bytes[8..12].copy_from_slice(&self._ref.buffer_index.to_le_bytes());
                bytes[12..16].copy_from_slice(&self._ref.offset.to_le_bytes());
            } else {
                bytes[4..16].copy_from_slice(&self.inlined.data);
            }
        }
        bytes
    }
}

pub const VIEW_SIZE: usize = std::mem::size_of::<BinaryView>();

#[derive(Debug, Clone)]
pub struct VarBinViewArray {
    views: Box<Array>,
    data: Vec<Array>,
    dtype: DType,
    stats: Arc<RwLock<StatsSet>>,
}

impl VarBinViewArray {
    pub fn new(views: Box<Array>, data: Vec<Array>, dtype: DType) -> Self {
        if !matches!(
            views.dtype(),
            DType::Int(IntWidth::_8, Signedness::Unsigned)
        ) {
            panic!("Unsupported type for views array {:?}", views.dtype());
        }
        data.iter().for_each(|d| {
            if !matches!(d.dtype(), DType::Int(IntWidth::_8, Signedness::Unsigned)) {
                panic!("Unsupported type for data array {:?}", d.dtype());
            }
        });
        if !matches!(dtype, DType::Binary | DType::Utf8) {
            panic!("Unsupported dtype for VarBinView array");
        }

        Self {
            views,
            data,
            dtype,
            stats: Arc::new(RwLock::new(StatsSet::new())),
        }
    }

    pub fn plain_size(&self) -> usize {
        (0..self.views.len() / VIEW_SIZE).fold(0usize, |acc, i| {
            let view = self.view_at(i);
            unsafe { acc + view.inlined.size as usize }
        })
    }

    #[inline]
    pub(self) fn view_at(&self, index: usize) -> BinaryView {
        let view_slice = self
            .views
            .slice(index * VIEW_SIZE, (index + 1) * VIEW_SIZE)
            .unwrap()
            .iter_arrow()
            .combine_chunks();
        let view_vec = view_slice.as_primitive::<UInt8Type>().values().to_vec();
        BinaryView::from_le_bytes(&view_vec)
    }
}

impl ArrayEncoding for VarBinViewArray {
    #[inline]
    fn len(&self) -> usize {
        self.views.len() / std::mem::size_of::<BinaryView>()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    #[inline]
    fn dtype(&self) -> &DType {
        &self.dtype
    }

    #[inline]
    fn stats(&self) -> Stats {
        Stats::new(&self.stats, self)
    }

    fn scalar_at(&self, index: usize) -> EncResult<Box<dyn Scalar>> {
        let view = self.view_at(index);
        unsafe {
            let value_bytes = if view.inlined.size > 12 {
                let arrow_data_buffer = self
                    .data
                    .get(view._ref.buffer_index as usize)
                    .unwrap()
                    .slice(
                        view._ref.offset as usize,
                        (view._ref.size + view._ref.offset) as usize,
                    )?
                    .iter_arrow()
                    .combine_chunks();

                arrow_data_buffer
                    .as_primitive::<UInt8Type>()
                    .values()
                    .to_vec()
            } else {
                view.inlined.data[..view.inlined.size as usize].to_vec()
            };

            if matches!(self.dtype, DType::Utf8) {
                Ok(Utf8Scalar::new(String::from_utf8_unchecked(value_bytes)).boxed())
            } else {
                Ok(BinaryScalar::new(value_bytes).boxed())
            }
        }
    }

    // TODO(robert): This could be better if we had compute dispatch but for now it's using scalar_at
    // and wraps values needlessly instead of memcopy
    fn iter_arrow(&self) -> Box<ArrowIterator> {
        let mut data_buf = StringBuilder::with_capacity(self.len(), self.plain_size());
        for i in 0..self.views.len() / VIEW_SIZE {
            data_buf.append_value(
                self.scalar_at(i)
                    .unwrap()
                    .as_any()
                    .downcast_ref::<Utf8Scalar>()
                    .unwrap()
                    .value(),
            );
        }
        let data_arr: ArrayRef = Arc::new(data_buf.finish());
        Box::new(iter::once(data_arr))
    }

    fn slice(&self, start: usize, stop: usize) -> EncResult<Array> {
        self.check_slice_bounds(start, stop)?;

        Ok(Array::VarBinView(Self {
            views: Box::new(self.views.slice(start * VIEW_SIZE, stop * VIEW_SIZE)?),
            data: self.data.clone(),
            dtype: self.dtype.clone(),
            stats: Arc::new(RwLock::new(StatsSet::new())),
        }))
    }
}

#[cfg(test)]
mod test {
    use arrow::array::GenericStringArray as ArrowStringArray;

    use crate::array::primitive::PrimitiveArray;

    use super::*;

    fn binary_array() -> VarBinViewArray {
        let values =
            PrimitiveArray::from_vec("hello world this is a long string".as_bytes().to_vec());
        let view1 = BinaryView {
            inlined: Inlined::new("hello world"),
        };
        let view2 = BinaryView {
            _ref: Ref {
                size: 33,
                prefix: "hell".as_bytes().try_into().unwrap(),
                buffer_index: 0,
                offset: 0,
            },
        };
        let view_arr = PrimitiveArray::from_vec(
            vec![view1.to_le_bytes(), view2.to_le_bytes()]
                .into_iter()
                .flatten()
                .collect::<Vec<u8>>(),
        );

        VarBinViewArray::new(Box::new(view_arr.into()), vec![values.into()], DType::Utf8)
    }

    #[test]
    pub fn varbin_view() {
        let binary_arr = binary_array();
        assert_eq!(binary_arr.len(), 2);
        assert_eq!(
            binary_arr.scalar_at(0).unwrap(),
            Utf8Scalar::new("hello world".into()).boxed()
        );
        assert_eq!(
            binary_arr.scalar_at(1).unwrap(),
            Utf8Scalar::new("hello world this is a long string".into()).boxed()
        )
    }

    #[test]
    pub fn slice() {
        let binary_arr = binary_array().slice(1, 2).unwrap();
        assert_eq!(
            binary_arr.scalar_at(0).unwrap(),
            Utf8Scalar::new("hello world this is a long string".into()).boxed()
        );
    }

    #[test]
    pub fn iter() {
        let binary_array = binary_array();
        assert_eq!(
            binary_array
                .iter_arrow()
                .combine_chunks()
                .as_string::<i32>(),
            &ArrowStringArray::<i32>::from(vec![
                "hello world",
                "hello world this is a long string"
            ])
        );
    }
}