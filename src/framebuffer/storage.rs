use zstd;

use framebuffer::common;
use ndarray::Array2;
use std::sync::Arc;

#[derive(Clone)]
pub struct CompressedCanvasState {
    data: Arc<[u8]>,
    rows: usize,
    cols: usize,
}

/// For reference, a rectangle with height=1050 and width=1404
/// will have the following size at rest:
///
/// (notice better compression at first due to relatively low-entropy canvas
///  compression plateuaus around 93% as the entropy peaks)
///
///    raw: 5896.8 kB -- zstd: 7.875 kB  (99.86645% compression)
///    raw: 5896.8 kB -- zstd: 25.405 kB  (99.569176% compression)
///    raw: 5896.8 kB -- zstd: 210.628 kB  (96.4281% compression)
///    raw: 5896.8 kB -- zstd: 367.217 kB  (93.7726% compression)
///    raw: 5896.8 kB -- zstd: 367.217 kB  (93.7726% compression)
///    raw: 5896.8 kB -- zstd: 356.432 kB  (93.9555% compression)
///    raw: 5896.8 kB -- zstd: 361.935 kB  (93.86218% compression)
impl CompressedCanvasState {

    /// Creates a CompressedCanvasState from the output of FramebufferIO::dump_region(..)
    pub fn new(arr: &Array2<common::color>) -> CompressedCanvasState {
        let mut unencoded: Vec<u8> = Vec::new();
        for row_index in 0..arr.rows() {
            let row = arr.row(row_index);
            for col in 0..row.len() {
                let color = row.get(col).unwrap().as_native();
                unencoded.push(color[0]);
                unencoded.push(color[1]);
                unencoded.push(color[2]);
                unencoded.push(color[3]);
            }
        }
        return CompressedCanvasState {
            data: zstd::encode_all(unencoded.as_slice(), 0)
                .unwrap()
                .into(),
            rows: arr.rows(),
            cols: arr.cols(),
        };
    }

    /// Returns an Arc<Array2<common::color>> which can be used to restore the contents of a screen
    /// region using the FramebufferIO::restore_region(..)
    pub fn decompress(&self) -> Arc<Array2<common::color>> {
        let unencoded = zstd::decode_all(&*self.data).unwrap();
        let mut output = Array2::default((self.rows, self.cols));

        for y_offset in 0..self.rows {
            let mut row = output.row_mut(y_offset as usize);
            for x_offset in 0..self.cols {
                let base = (y_offset * self.cols + x_offset) * 4;
                let mut pixel = row.get_mut(x_offset).unwrap();
                *pixel = common::color::from_native([
                    unencoded[base],
                    unencoded[base + 1],
                    unencoded[base + 2],
                    unencoded[base + 3],
                ]);
            }
        }
        return Arc::new(output);
    }
}
