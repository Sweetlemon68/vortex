use std::marker::PhantomData;
use std::sync::Arc;

use arrow_buffer::Buffer;
use vortex_error::{vortex_bail, VortexError, VortexResult};
use vortex_schema::DType;

use crate::encoding::EncodingRef;
use crate::{Array, ArrayDef, ArrayMetadata, ArrayParts, IntoArray, ToArray};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ArrayData {
    encoding: EncodingRef,
    dtype: DType,
    metadata: Arc<dyn ArrayMetadata>,
    buffers: Arc<[Buffer]>, // Should this just be an Option, not an Arc?
    children: Arc<[Option<ArrayData>]>,
}

impl ArrayData {
    pub fn try_new(
        encoding: EncodingRef,
        dtype: DType,
        metadata: Arc<dyn ArrayMetadata>,
        buffers: Arc<[Buffer]>,
        children: Arc<[Option<ArrayData>]>,
    ) -> VortexResult<Self> {
        let data = Self {
            encoding,
            dtype,
            metadata,
            buffers,
            children,
        };

        // Validate here that the metadata correctly parses, so that an encoding can infallibly
        // implement Encoding::with_data().
        // encoding.with_data_mut(&data, &mut |_| Ok(()))?;

        Ok(data)
    }
}

impl ArrayData {
    pub fn encoding(&self) -> EncodingRef {
        self.encoding
    }

    pub fn dtype(&self) -> &DType {
        &self.dtype
    }

    pub fn metadata(&self) -> &Arc<dyn ArrayMetadata> {
        &self.metadata
    }

    pub fn buffers(&self) -> &[Buffer] {
        &self.buffers
    }

    pub fn child(&self, index: usize) -> Option<&ArrayData> {
        self.children.get(index).and_then(|c| c.as_ref())
    }
}

impl ToArray for ArrayData {
    fn to_array(&self) -> Array {
        Array::DataRef(self)
    }
}

impl IntoArray<'static> for ArrayData {
    fn into_array(self) -> Array<'static> {
        Array::Data(self)
    }
}

#[derive(Debug)]
pub struct TypedArrayData<D: ArrayDef> {
    data: ArrayData,
    phantom: PhantomData<D>,
}

impl<D: ArrayDef> TypedArrayData<D> {
    pub fn new_unchecked(
        dtype: DType,
        metadata: Arc<D::Metadata>,
        buffers: Arc<[Buffer]>,
        children: Arc<[Option<ArrayData>]>,
    ) -> Self {
        Self::from_data_unchecked(
            ArrayData::try_new(D::ENCODING, dtype, metadata, buffers, children).unwrap(),
        )
    }

    pub fn from_data_unchecked(data: ArrayData) -> Self {
        Self {
            data,
            phantom: PhantomData,
        }
    }

    pub fn data(&self) -> &ArrayData {
        &self.data
    }

    pub fn into_data(self) -> ArrayData {
        self.data
    }

    pub fn metadata(&self) -> &D::Metadata {
        self.data
            .metadata()
            .as_any()
            .downcast_ref::<D::Metadata>()
            .unwrap()
    }

    pub fn into_metadata(self) -> Arc<D::Metadata> {
        self.data
            .metadata
            .as_any_arc()
            .downcast::<D::Metadata>()
            .unwrap()
    }
}

impl<D: ArrayDef> ToArray for TypedArrayData<D> {
    fn to_array(&self) -> Array {
        Array::DataRef(&self.data)
    }
}

impl<D: ArrayDef> IntoArray<'static> for TypedArrayData<D> {
    fn into_array(self) -> Array<'static> {
        Array::Data(self.data)
    }
}

impl<D: ArrayDef> TryFrom<ArrayData> for TypedArrayData<D> {
    type Error = VortexError;

    fn try_from(data: ArrayData) -> Result<Self, Self::Error> {
        if data.encoding().id() != D::ID {
            vortex_bail!("Invalid encoding for array")
        }
        Ok(Self {
            data,
            phantom: PhantomData,
        })
    }
}

impl ArrayParts<'_> for ArrayData {
    fn dtype(&'_ self) -> &'_ DType {
        &self.dtype
    }

    fn buffer(&self, idx: usize) -> Option<&Buffer> {
        self.buffers().get(idx)
    }

    fn child(&self, idx: usize, _dtype: &DType) -> Option<Array> {
        self.child(idx).map(|a| {
            let array = a.to_array();
            // FIXME(ngates): can we ask an array its dtype?
            // assert_eq!(array.dtype(), dtype);
            array
        })
    }
}