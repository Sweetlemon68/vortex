// run on top of Compio files

use std::future::Future;
use std::io;

use bytes::BytesMut;
use compio::fs::File;
use compio::io::AsyncReadAtExt;
use compio::runtime::Runtime;
use compio::BufResult;
use vortex_error::vortex_panic;

use super::VortexReadAt;
use crate::file::AsyncRuntime;

pub struct CompioAdapter<IO>(IO);

impl VortexReadAt for File {
    fn read_at_into(
        &self,
        pos: u64,
        buffer: BytesMut,
    ) -> impl Future<Output = io::Result<BytesMut>> + 'static {
        let this = self.clone();
        async move {
            // Turn the buffer into a static slice.
            let BufResult(res, buffer) = this.read_exact_at(buffer, pos).await;
            res.map(|_| buffer)
        }
    }

    fn size(&self) -> impl Future<Output = u64> + 'static {
        let this = self.clone();
        async move {
            this.metadata()
                .await
                .map(|metadata| metadata.len())
                .unwrap_or_else(|e| vortex_panic!("compio File::size: {e}"))
        }
    }
}

impl AsyncRuntime for Runtime {
    fn block_on<F: Future>(&self, fut: F) -> F::Output {
        self.block_on(fut)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use bytes::BytesMut;
    use compio::fs::File;
    use tempfile::NamedTempFile;

    use crate::io::VortexReadAt;

    #[cfg_attr(miri, ignore)]
    #[compio::test]
    async fn test_read_at_compio_file() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        write!(tmpfile, "0123456789").unwrap();

        // Open up a file handle in compio land
        let file = File::open(tmpfile.path()).await.unwrap();

        // Use the file as a VortexReadAt instance.
        let four_bytes = BytesMut::zeroed(4);
        let read = file.read_at_into(2, four_bytes).await.unwrap();
        assert_eq!(&read, "2345".as_bytes());
    }
}