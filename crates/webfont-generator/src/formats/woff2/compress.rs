use std::io::{Cursor, Error};

use brotli::enc::backward_references::BrotliEncoderMode;
use brotli::enc::{
    BrotliCompress, BrotliEncoderMaxCompressedSizeMulti, BrotliEncoderParams, Owned, SendAlloc,
    SliceWrapper, StandardAlloc, UnionHasher,
};

use super::glyf::invalid_data;
#[cfg(any(test, feature = "bench"))]
use super::prepare::PreparedWoff2;

#[cfg(any(test, feature = "bench"))]
pub(super) fn compress(prepared: &PreparedWoff2, quality: u8) -> Result<Vec<u8>, Error> {
    compress_stream(prepared.stream.clone(), quality)
}

pub(super) fn compress_stream(stream: Vec<u8>, quality: u8) -> Result<Vec<u8>, Error> {
    dropbox_brotli_compress(stream, quality.min(11))
}

struct BrotliInput(Vec<u8>);

impl SliceWrapper<u8> for BrotliInput {
    fn slice(&self) -> &[u8] {
        &self.0
    }
}

fn dropbox_brotli_compress(input: Vec<u8>, quality: u8) -> Result<Vec<u8>, Error> {
    let params = BrotliEncoderParams {
        mode: BrotliEncoderMode::BROTLI_MODE_FONT,
        quality: i32::from(quality),
        lgwin: 22,
        size_hint: input.len(),
        ..Default::default()
    };

    if quality < 10 {
        let mut output = Vec::new();
        BrotliCompress(&mut Cursor::new(&input), &mut output, &params)?;
        return Ok(output);
    }

    const THREADS: usize = 2;
    let mut output = vec![0; BrotliEncoderMaxCompressedSizeMulti(input.len(), THREADS)];
    let mut allocators = (0..THREADS)
        .map(|_| SendAlloc::new(StandardAlloc::default(), UnionHasher::Uninit))
        .collect::<Vec<_>>();
    let length = brotli::enc::compress_multi(
        &params,
        &mut Owned::new(BrotliInput(input)),
        &mut output,
        &mut allocators,
    )
    .map_err(|_| invalid_data("Brotli compression failed"))?;
    output.truncate(length);
    Ok(output)
}
