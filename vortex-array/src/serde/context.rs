use std::sync::Arc;

use itertools::Itertools;

use crate::encoding::{EncodingId, EncodingRef, ENCODINGS};

#[derive(Debug)]
pub struct SerdeContext {
    encodings: Arc<[EncodingRef]>,
}

impl SerdeContext {
    pub fn new(encodings: Arc<[EncodingRef]>) -> Self {
        Self { encodings }
    }

    pub fn encodings(&self) -> &[EncodingRef] {
        self.encodings.as_ref()
    }

    pub fn find_encoding(&self, encoding_id: u16) -> Option<EncodingRef> {
        self.encodings.get(encoding_id as usize).cloned()
    }

    pub fn encoding_idx(&self, encoding_id: EncodingId) -> Option<u16> {
        self.encodings
            .iter()
            .position(|e| e.id() == encoding_id)
            .map(|i| i as u16)
    }
}

impl Default for SerdeContext {
    fn default() -> Self {
        Self {
            encodings: ENCODINGS.iter().cloned().collect_vec().into(),
        }
    }
}
