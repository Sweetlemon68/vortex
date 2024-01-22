use crate::error::EncResult;
use crate::scalar::Scalar;
use crate::types::DType;
use std::any::Any;

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryScalar {
    value: Vec<u8>,
}

impl BinaryScalar {
    pub fn new(value: Vec<u8>) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &Vec<u8> {
        &self.value
    }
}

impl Scalar for BinaryScalar {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn boxed(self) -> Box<dyn Scalar> {
        Box::new(self)
    }
    #[inline]
    fn dtype(&self) -> &DType {
        &DType::Binary
    }

    fn cast(&self, _dtype: &DType) -> EncResult<Box<dyn Scalar>> {
        todo!()
    }
}