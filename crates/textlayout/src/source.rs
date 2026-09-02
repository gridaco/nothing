//! Complete source coverage, opaque caller association, and explicit shaping
//! segmentation.
//!
//! Source runs are metadata over the exact UTF-8 string. Their tags are
//! deliberately meaningless to this crate: resolution carries them from
//! source spans to shaping clusters and glyphs, but never interprets them as
//! color, style, or any render-contract value.
//!
//! Shaping chunks are geometry-producing input. Each chunk is sent through
//! the shaper independently, so its boundaries suppress every shaping
//! interaction across them. They carry no authored-language placement or
//! anchoring meaning.

use std::ops::Range;

/// An opaque caller-defined association for one or more source runs.
///
/// Equal tags intentionally mean the same caller selection even when they
/// occur in disjoint or adjacent runs. The numeric value has no semantics to
/// `textlayout`; it only lets a caller recover its own selection after the
/// complete string has been shaped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceRunTag(u32);

impl SourceRunTag {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

/// One caller-associated span of authored source in UTF-8 byte coordinates.
///
/// Construction does not claim validity. [`crate::resolve`] validates the
/// complete list before shaping so malformed, empty, gapped, overlapping, or
/// non-scalar-boundary coverage returns a typed error instead of being
/// repaired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRun {
    source_utf8: Range<usize>,
    tag: SourceRunTag,
}

/// One independently shaped span of source in UTF-8 byte coordinates.
///
/// Construction does not claim validity. [`crate::resolve`] requires chunks
/// to be a complete ordered partition of the source on scalar boundaries and
/// returns a typed coverage error before font work if they are malformed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapingChunk {
    source_utf8: Range<usize>,
}

impl ShapingChunk {
    pub fn new(source_utf8: Range<usize>) -> Self {
        Self { source_utf8 }
    }

    pub fn source_utf8(&self) -> Range<usize> {
        self.source_utf8.clone()
    }
}

impl SourceRun {
    pub fn new(source_utf8: Range<usize>, tag: SourceRunTag) -> Self {
        Self { source_utf8, tag }
    }

    pub fn source_utf8(&self) -> Range<usize> {
        self.source_utf8.clone()
    }

    pub const fn tag(&self) -> SourceRunTag {
        self.tag
    }
}

/// The precise reason source-run coverage is invalid.
///
/// This is part of the producer contract: callers may classify invalid
/// attributed source without parsing diagnostic text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceRunCoverageError {
    /// A non-empty source has no source runs.
    Missing { source_len: usize },
    /// One run points backwards.
    Reversed {
        run_index: usize,
        start: usize,
        end: usize,
    },
    /// Empty runs carry no source scalar and are never meaningful coverage.
    Empty { run_index: usize, byte_index: usize },
    /// A run reaches outside the source string.
    OutOfBounds {
        run_index: usize,
        start: usize,
        end: usize,
        source_len: usize,
    },
    /// A run starts or ends inside a multi-byte UTF-8 scalar.
    NotScalarBoundary { run_index: usize, byte_index: usize },
    /// A run begins after the byte where contiguous coverage must continue.
    Gap {
        run_index: usize,
        expected_start: usize,
        actual_start: usize,
    },
    /// A run begins before the preceding run ended.
    Overlap {
        run_index: usize,
        previous_end: usize,
        actual_start: usize,
    },
    /// The final run ends before the source does.
    Incomplete {
        covered_end: usize,
        source_len: usize,
    },
}

/// The precise reason shaping-chunk coverage is invalid.
///
/// This is distinct from [`SourceRunCoverageError`]: source runs are opaque
/// associations, while shaping chunks alter geometry. A caller can classify
/// either malformed input without parsing diagnostic text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShapingChunkCoverageError {
    /// A non-empty source has no shaping chunks.
    Missing { source_len: usize },
    /// One chunk points backwards.
    Reversed {
        chunk_index: usize,
        start: usize,
        end: usize,
    },
    /// Empty chunks cannot produce an independent shaping result.
    Empty {
        chunk_index: usize,
        byte_index: usize,
    },
    /// A chunk reaches outside the source string.
    OutOfBounds {
        chunk_index: usize,
        start: usize,
        end: usize,
        source_len: usize,
    },
    /// A chunk starts or ends inside a multi-byte UTF-8 scalar.
    NotScalarBoundary {
        chunk_index: usize,
        byte_index: usize,
    },
    /// A chunk begins after the byte where contiguous coverage must continue.
    Gap {
        chunk_index: usize,
        expected_start: usize,
        actual_start: usize,
    },
    /// A chunk begins before the preceding chunk ended.
    Overlap {
        chunk_index: usize,
        previous_end: usize,
        actual_start: usize,
    },
    /// The final chunk ends before the source does.
    Incomplete {
        covered_end: usize,
        source_len: usize,
    },
}

impl std::fmt::Display for SourceRunCoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { source_len } => {
                write!(
                    f,
                    "non-empty source of {source_len} bytes has no source runs"
                )
            }
            Self::Reversed {
                run_index,
                start,
                end,
            } => write!(f, "source run {run_index} is reversed ({start}..{end})"),
            Self::Empty {
                run_index,
                byte_index,
            } => write!(f, "source run {run_index} is empty at byte {byte_index}"),
            Self::OutOfBounds {
                run_index,
                start,
                end,
                source_len,
            } => write!(
                f,
                "source run {run_index} ({start}..{end}) exceeds the {source_len}-byte source"
            ),
            Self::NotScalarBoundary {
                run_index,
                byte_index,
            } => write!(
                f,
                "source run {run_index} has a boundary inside a UTF-8 scalar at byte {byte_index}"
            ),
            Self::Gap {
                run_index,
                expected_start,
                actual_start,
            } => write!(
                f,
                "source run {run_index} starts at byte {actual_start}, leaving a gap after byte {expected_start}"
            ),
            Self::Overlap {
                run_index,
                previous_end,
                actual_start,
            } => write!(
                f,
                "source run {run_index} starts at byte {actual_start}, overlapping coverage through byte {previous_end}"
            ),
            Self::Incomplete {
                covered_end,
                source_len,
            } => write!(
                f,
                "source-run coverage ends at byte {covered_end} before the {source_len}-byte source ends"
            ),
        }
    }
}

impl std::fmt::Display for ShapingChunkCoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { source_len } => {
                write!(
                    f,
                    "non-empty source of {source_len} bytes has no shaping chunks"
                )
            }
            Self::Reversed {
                chunk_index,
                start,
                end,
            } => write!(
                f,
                "shaping chunk {chunk_index} is reversed ({start}..{end})"
            ),
            Self::Empty {
                chunk_index,
                byte_index,
            } => write!(
                f,
                "shaping chunk {chunk_index} is empty at byte {byte_index}"
            ),
            Self::OutOfBounds {
                chunk_index,
                start,
                end,
                source_len,
            } => write!(
                f,
                "shaping chunk {chunk_index} ({start}..{end}) exceeds the {source_len}-byte source"
            ),
            Self::NotScalarBoundary {
                chunk_index,
                byte_index,
            } => write!(
                f,
                "shaping chunk {chunk_index} has a boundary inside a UTF-8 scalar at byte {byte_index}"
            ),
            Self::Gap {
                chunk_index,
                expected_start,
                actual_start,
            } => write!(
                f,
                "shaping chunk {chunk_index} starts at byte {actual_start}, leaving a gap after byte {expected_start}"
            ),
            Self::Overlap {
                chunk_index,
                previous_end,
                actual_start,
            } => write!(
                f,
                "shaping chunk {chunk_index} starts at byte {actual_start}, overlapping coverage through byte {previous_end}"
            ),
            Self::Incomplete {
                covered_end,
                source_len,
            } => write!(
                f,
                "shaping-chunk coverage ends at byte {covered_end} before the {source_len}-byte source ends"
            ),
        }
    }
}

/// Validate exact, ordered source coverage before any font work or shaping.
pub(crate) fn validate_source_runs(
    source: &str,
    runs: &[SourceRun],
) -> Result<(), SourceRunCoverageError> {
    if source.is_empty() && runs.is_empty() {
        return Ok(());
    }
    if runs.is_empty() {
        return Err(SourceRunCoverageError::Missing {
            source_len: source.len(),
        });
    }

    let mut covered_end = 0usize;
    for (run_index, run) in runs.iter().enumerate() {
        let range = run.source_utf8();
        if range.start > range.end {
            return Err(SourceRunCoverageError::Reversed {
                run_index,
                start: range.start,
                end: range.end,
            });
        }
        if range.is_empty() {
            return Err(SourceRunCoverageError::Empty {
                run_index,
                byte_index: range.start,
            });
        }
        if range.start > source.len() || range.end > source.len() {
            return Err(SourceRunCoverageError::OutOfBounds {
                run_index,
                start: range.start,
                end: range.end,
                source_len: source.len(),
            });
        }
        for byte_index in [range.start, range.end] {
            if !source.is_char_boundary(byte_index) {
                return Err(SourceRunCoverageError::NotScalarBoundary {
                    run_index,
                    byte_index,
                });
            }
        }
        if range.start > covered_end {
            return Err(SourceRunCoverageError::Gap {
                run_index,
                expected_start: covered_end,
                actual_start: range.start,
            });
        }
        if range.start < covered_end {
            return Err(SourceRunCoverageError::Overlap {
                run_index,
                previous_end: covered_end,
                actual_start: range.start,
            });
        }
        covered_end = range.end;
    }

    if covered_end != source.len() {
        return Err(SourceRunCoverageError::Incomplete {
            covered_end,
            source_len: source.len(),
        });
    }
    Ok(())
}

/// Validate exact, ordered shaping segmentation before font work.
pub(crate) fn validate_shaping_chunks(
    source: &str,
    chunks: &[ShapingChunk],
) -> Result<(), ShapingChunkCoverageError> {
    if source.is_empty() && chunks.is_empty() {
        return Ok(());
    }
    if chunks.is_empty() {
        return Err(ShapingChunkCoverageError::Missing {
            source_len: source.len(),
        });
    }

    let mut covered_end = 0usize;
    for (chunk_index, chunk) in chunks.iter().enumerate() {
        let range = chunk.source_utf8();
        if range.start > range.end {
            return Err(ShapingChunkCoverageError::Reversed {
                chunk_index,
                start: range.start,
                end: range.end,
            });
        }
        if range.is_empty() {
            return Err(ShapingChunkCoverageError::Empty {
                chunk_index,
                byte_index: range.start,
            });
        }
        if range.start > source.len() || range.end > source.len() {
            return Err(ShapingChunkCoverageError::OutOfBounds {
                chunk_index,
                start: range.start,
                end: range.end,
                source_len: source.len(),
            });
        }
        for byte_index in [range.start, range.end] {
            if !source.is_char_boundary(byte_index) {
                return Err(ShapingChunkCoverageError::NotScalarBoundary {
                    chunk_index,
                    byte_index,
                });
            }
        }
        if range.start > covered_end {
            return Err(ShapingChunkCoverageError::Gap {
                chunk_index,
                expected_start: covered_end,
                actual_start: range.start,
            });
        }
        if range.start < covered_end {
            return Err(ShapingChunkCoverageError::Overlap {
                chunk_index,
                previous_end: covered_end,
                actual_start: range.start,
            });
        }
        covered_end = range.end;
    }

    if covered_end != source.len() {
        return Err(ShapingChunkCoverageError::Incomplete {
            covered_end,
            source_len: source.len(),
        });
    }
    Ok(())
}

/// The run tag covering one source scalar's first byte.
pub(crate) fn source_run_tag_at(runs: &[SourceRun], byte_index: usize) -> SourceRunTag {
    runs.iter()
        .find(|run| run.source_utf8.contains(&byte_index))
        .map(SourceRun::tag)
        .expect("validated source coverage contains every source byte")
}
