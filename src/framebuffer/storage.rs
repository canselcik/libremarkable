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

impl CompressedCanvasState {
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

        // for reference, a rectangle with
        //   height: 1050,
        //   width: 1404,
        // will have the following size at rest:
        //   snapshot raw: 5896 kB
        //   zstd: 10 kB
        return CompressedCanvasState {
            data: zstd::encode_all(unencoded.as_slice(), 0)
                .unwrap()
                .into(),
            rows: arr.rows(),
            cols: arr.cols(),
        };
    }

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
