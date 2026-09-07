//! ENG S-6 · feature-gated observability. With the `trace` feature off,
//! [`t_span!`] expands to the bare block and [`t_count!`] to nothing —
//! zero cost, so the profiler can never distort the profile (the legacy
//! loop measured exactly this trap: devtools cost-prediction polluted
//! plan build until it was gated). With `trace` on, spans accumulate
//! into a thread-local [`sink`] the host or gate drains per frame.
//!
//! Instrument only the three frame seams (resolve / build / execute) —
//! more is noise. The on/off delta is a one-time documented measurement,
//! not a per-frame check.
//!
//! Blend-layer counters are separate from duration samples. With `trace`,
//! each outermost execute publishes one `BlendLayerMetrics` through
//! `sink::drain_blend_layers`, including recursive resource executions.

/// Diagnostic observations of blend layers during one outermost execution.
///
/// These are not allocator-capacity, GPU-memory, or default-build measurements.
/// Raster bytes describe the accessible pixel span (`ImageInfo::compute_byte_size`
/// with the observed row stride), excluding allocator overhead. The pinned Skia
/// accessor does not allocate pixels, but marks their generation changed even
/// when n0 only reads metadata. Use a separate untimed trace-enabled frame.
/// Missing observations make byte totals and the live peak incomplete.
/// Only explicit execute-seam blend scopes are counted: allocations internal
/// to Skia (including later playback of a recorded picture) are not observable
/// here. Preflight picture recording can produce separate execute aggregates.
#[cfg(feature = "trace")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BlendLayerMetrics {
    /// BeginIsolatedBlend save-layer calls, including empty clips.
    pub save_layer_calls: u64,
    /// Calls whose top-layer raster storage was observable.
    pub observed_raster_layers: u64,
    /// Sum of observed accessible pixel-span bytes, not retained memory.
    pub observed_raster_bytes: u128,
    /// Sum of observed raster width × height, in pixels.
    pub observed_raster_pixels: u128,
    /// Largest simultaneously live observed blend-layer byte total.
    /// Other opacity, mask, filter, and root-surface storage is excluded.
    pub peak_live_blend_bytes: u128,
    /// Nonempty-clip calls without accessible raster storage (for example GPU,
    /// recording canvases, failed allocations, or inaccessible layer mappings).
    pub missing_observations: u64,
    /// Empty-clip saves, which may leave the parent device on top; never counted
    /// as observing another allocation of that parent surface.
    pub empty_clip_saves: u64,
}

#[cfg(feature = "trace")]
pub mod sink {
    use std::cell::RefCell;

    thread_local! {
        static FRAME: RefCell<Vec<(&'static str, u128)>> = const { RefCell::new(Vec::new()) };
        static BLEND_LAYERS: RefCell<Vec<super::BlendLayerMetrics>> = const { RefCell::new(Vec::new()) };
    }

    /// Record a span sample (nanoseconds) under `name`.
    pub fn record(name: &'static str, nanos: u128) {
        FRAME.with(|f| f.borrow_mut().push((name, nanos)));
    }

    /// Take and clear this thread's accumulated spans.
    pub fn drain() -> Vec<(&'static str, u128)> {
        FRAME.with(|f| f.borrow_mut().drain(..).collect())
    }

    pub(super) fn record_blend_layers(metrics: super::BlendLayerMetrics) {
        BLEND_LAYERS.with(|frames| frames.borrow_mut().push(metrics));
    }

    /// Take and clear this thread's completed outermost-execute blend-layer
    /// observations. This does not drain duration samples or an active execute.
    pub fn drain_blend_layers() -> Vec<super::BlendLayerMetrics> {
        BLEND_LAYERS.with(|frames| frames.borrow_mut().drain(..).collect())
    }
}

#[cfg(feature = "trace")]
pub(crate) mod blend_layers {
    use std::cell::RefCell;

    use super::{sink, BlendLayerMetrics};

    #[derive(Default)]
    struct Active {
        depth: usize,
        live_bytes: u128,
        metrics: BlendLayerMetrics,
    }

    thread_local! {
        static ACTIVE: RefCell<Active> = RefCell::new(Active::default());
    }

    pub(crate) enum Observation {
        Raster { bytes: usize, pixels: u64 },
        EmptyClip,
        Unavailable,
    }

    // Recursive resource execution contributes to the same aggregate and sees
    // the outer layers' live bytes. No per-layer events are retained.
    pub(crate) struct Execute {
        entry_live_bytes: u128,
    }

    impl Execute {
        pub(crate) fn begin() -> Self {
            ACTIVE.with(|active| {
                let mut active = active.borrow_mut();
                active.depth += 1;
                Self {
                    entry_live_bytes: active.live_bytes,
                }
            })
        }

        pub(crate) fn begin_layer(observation: Observation) -> u128 {
            ACTIVE.with(|active| {
                let mut active = active.borrow_mut();
                debug_assert!(active.depth > 0);
                active.metrics.save_layer_calls += 1;
                match observation {
                    Observation::Raster { bytes, pixels } => {
                        let bytes = bytes as u128;
                        active.metrics.observed_raster_layers += 1;
                        active.metrics.observed_raster_bytes += bytes;
                        active.metrics.observed_raster_pixels += u128::from(pixels);
                        active.live_bytes += bytes;
                        active.metrics.peak_live_blend_bytes =
                            active.metrics.peak_live_blend_bytes.max(active.live_bytes);
                        bytes
                    }
                    Observation::EmptyClip => {
                        active.metrics.empty_clip_saves += 1;
                        0
                    }
                    Observation::Unavailable => {
                        active.metrics.missing_observations += 1;
                        0
                    }
                }
            })
        }

        pub(crate) fn end_layer(bytes: u128) {
            ACTIVE.with(|active| {
                let mut active = active.borrow_mut();
                debug_assert!(active.live_bytes >= bytes);
                active.live_bytes = active.live_bytes.saturating_sub(bytes);
            });
        }
    }

    impl Drop for Execute {
        fn drop(&mut self) {
            let completed = ACTIVE.with(|active| {
                let mut active = active.borrow_mut();
                active.depth -= 1;
                active.live_bytes = self.entry_live_bytes;
                if active.depth == 0 {
                    Some(std::mem::take(&mut active.metrics))
                } else {
                    None
                }
            });
            if let Some(metrics) = completed {
                if !std::thread::panicking() {
                    sink::record_blend_layers(metrics);
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn recursive_execution_aggregates_live_bytes_and_drains_separately() {
            sink::drain_blend_layers();
            sink::drain();
            sink::record("duration", 17);
            {
                let _outer = Execute::begin();
                let outer = Execute::begin_layer(Observation::Raster {
                    bytes: 20,
                    pixels: 5,
                });
                {
                    let _resource = Execute::begin();
                    let inner = Execute::begin_layer(Observation::Raster {
                        bytes: 32,
                        pixels: 8,
                    });
                    Execute::end_layer(inner);
                }
                assert!(sink::drain_blend_layers().is_empty());
                Execute::end_layer(outer);
                let sibling = Execute::begin_layer(Observation::Raster {
                    bytes: 12,
                    pixels: 3,
                });
                Execute::end_layer(sibling);
            }
            assert_eq!(
                sink::drain_blend_layers(),
                [BlendLayerMetrics {
                    save_layer_calls: 3,
                    observed_raster_layers: 3,
                    observed_raster_bytes: 64,
                    observed_raster_pixels: 16,
                    peak_live_blend_bytes: 52,
                    ..BlendLayerMetrics::default()
                }]
            );
            assert!(sink::drain_blend_layers().is_empty());
            assert_eq!(sink::drain(), [("duration", 17)]);
            drop(Execute::begin());
            assert_eq!(sink::drain_blend_layers(), [BlendLayerMetrics::default()]);
        }

        #[test]
        fn unavailable_and_empty_observations_do_not_invent_storage() {
            sink::drain_blend_layers();
            {
                let _execute = Execute::begin();
                Execute::end_layer(Execute::begin_layer(Observation::Unavailable));
                Execute::end_layer(Execute::begin_layer(Observation::EmptyClip));
            }
            assert_eq!(
                sink::drain_blend_layers(),
                [BlendLayerMetrics {
                    save_layer_calls: 2,
                    missing_observations: 1,
                    empty_clip_saves: 1,
                    ..BlendLayerMetrics::default()
                }]
            );
        }

        #[test]
        fn observations_are_thread_local() {
            sink::drain_blend_layers();
            let metrics = std::thread::spawn(|| {
                drop(Execute::begin());
                sink::drain_blend_layers()
            })
            .join()
            .unwrap();
            assert_eq!(metrics, [BlendLayerMetrics::default()]);
            assert!(sink::drain_blend_layers().is_empty());
        }
    }
}

/// Time `$body` under `$name`. Off: just the block. On: the block, plus a
/// sample into the thread-local sink. Value-transparent either way.
#[macro_export]
macro_rules! t_span {
    ($name:expr, $body:block) => {{
        #[cfg(feature = "trace")]
        {
            let __t0 = std::time::Instant::now();
            let __r = $body;
            $crate::trace::sink::record($name, __t0.elapsed().as_nanos());
            __r
        }
        #[cfg(not(feature = "trace"))]
        {
            $body
        }
    }};
}

/// Count an event under `$name`. Off: nothing. On: one nanosecond-free
/// sample (count lives in the same sink as a zero-duration marker).
#[macro_export]
macro_rules! t_count {
    ($name:expr) => {{
        #[cfg(feature = "trace")]
        {
            $crate::trace::sink::record($name, 0);
        }
    }};
}
