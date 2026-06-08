use std::{fmt, io, str::FromStr};

use thiserror::Error;

pub(crate) type CompressionResult<T> = Result<T, CompressionError>;

/// Error returned while compressing or decompressing stored bytes.
#[derive(Debug, Error)]
pub(crate) enum CompressionError {
    #[error("unsupported compression algorithm: {0}")]
    UnsupportedAlgo(String),
    #[error("invalid compression options for {algo}: {source}")]
    InvalidOptions {
        algo: CompressionAlgo,
        source: serde_json::Error,
    },
    #[error("compression failed for {algo}: {source}")]
    Compress {
        algo: CompressionAlgo,
        source: io::Error,
    },
    #[error("decompression failed for {algo}: {source}")]
    Decompress {
        algo: CompressionAlgo,
        source: io::Error,
    },
}

/// Compression algorithm used for persisted bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompressionAlgo {
    Zstd,
}

impl CompressionAlgo {
    const ZSTD: &'static str = "zstd";

    /// Returns the stable storage value for this algorithm.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Zstd => Self::ZSTD,
        }
    }
}

impl fmt::Display for CompressionAlgo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CompressionAlgo {
    type Err = CompressionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            Self::ZSTD => Ok(Self::Zstd),
            value => Err(CompressionError::UnsupportedAlgo(value.to_owned())),
        }
    }
}

/// Compresses and decompresses byte payloads.
pub(crate) trait CompressionCodec {
    /// Compresses bytes using the requested algorithm.
    fn compress(&self, algo: CompressionAlgo, bytes: &[u8]) -> CompressionResult<CompressedBytes>;

    /// Decompresses stored bytes using the recorded algorithm.
    fn decompress(
        &self,
        algo: CompressionAlgo,
        stored: StoredCompressedBytes<'_>,
    ) -> CompressionResult<Vec<u8>>;
}

/// Default compression codec used by `SQLite` persistence.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DefaultCompressionCodec;

impl CompressionCodec for DefaultCompressionCodec {
    fn compress(&self, algo: CompressionAlgo, bytes: &[u8]) -> CompressionResult<CompressedBytes> {
        match algo {
            CompressionAlgo::Zstd => zstd::compress(bytes),
        }
    }

    fn decompress(
        &self,
        algo: CompressionAlgo,
        stored: StoredCompressedBytes<'_>,
    ) -> CompressionResult<Vec<u8>> {
        match algo {
            CompressionAlgo::Zstd => zstd::decompress(stored),
        }
    }
}

/// Compressed bytes and options ready for persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompressedBytes {
    opts_json: String,
    bytes: Vec<u8>,
}

impl CompressedBytes {
    /// Returns the encoded compression options.
    pub(crate) fn opts_json(&self) -> &str {
        &self.opts_json
    }

    /// Returns the compressed bytes.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the compressed byte length.
    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    fn new(opts_json: String, bytes: Vec<u8>) -> Self {
        Self { opts_json, bytes }
    }
}

/// Stored compressed bytes and metadata needed to decode them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StoredCompressedBytes<'a> {
    pub(crate) opts_json: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) uncompressed_len: usize,
}

mod zstd {
    use serde::{Deserialize, Serialize};

    use super::{
        CompressedBytes, CompressionAlgo, CompressionError, CompressionResult,
        StoredCompressedBytes,
    };

    const LEVEL: i32 = 3;

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    struct ZstdOptions {
        level: i32,
    }

    pub(super) fn compress(bytes: &[u8]) -> CompressionResult<CompressedBytes> {
        let compressed =
            ::zstd::bulk::compress(bytes, LEVEL).map_err(|source| CompressionError::Compress {
                algo: CompressionAlgo::Zstd,
                source,
            })?;
        let opts = serde_json::to_string(&ZstdOptions { level: LEVEL }).map_err(|source| {
            CompressionError::InvalidOptions {
                algo: CompressionAlgo::Zstd,
                source,
            }
        })?;
        Ok(CompressedBytes::new(opts, compressed))
    }

    pub(super) fn decompress(stored: StoredCompressedBytes<'_>) -> CompressionResult<Vec<u8>> {
        let _opts = serde_json::from_str::<ZstdOptions>(stored.opts_json).map_err(|source| {
            CompressionError::InvalidOptions {
                algo: CompressionAlgo::Zstd,
                source,
            }
        })?;
        ::zstd::bulk::decompress(stored.bytes, stored.uncompressed_len).map_err(|source| {
            CompressionError::Decompress {
                algo: CompressionAlgo::Zstd,
                source,
            }
        })
    }
}
