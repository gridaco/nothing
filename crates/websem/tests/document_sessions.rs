//! Web producer lock for independently owned cascade sessions.
//!
//! The csscascade producer proves retained-handle isolation directly. This
//! consumer test proves that websem no longer serializes or selects documents
//! through process-global state: two host threads repeatedly compile sources
//! with colliding arena layouts but different visual facts.

use std::sync::{Arc, Barrier};

use cg::CGColor;
use websem::compile_standalone_svg;

const FIRST: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
  <rect width="16" height="16" fill="#112233"/>
</svg>
"##;

const SECOND: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
  <rect width="16" height="16" fill="#aabbcc"/>
</svg>
"##;

#[test]
fn independent_compiles_do_not_share_document_state() {
    let barrier = Arc::new(Barrier::new(2));
    let first = spawn_compile_loop(
        Arc::clone(&barrier),
        FIRST,
        CGColor::from_rgb(0x11, 0x22, 0x33),
    );
    let second = spawn_compile_loop(
        Arc::clone(&barrier),
        SECOND,
        CGColor::from_rgb(0xaa, 0xbb, 0xcc),
    );

    first.join().expect("first independent compile thread");
    second.join().expect("second independent compile thread");
}

fn spawn_compile_loop(
    barrier: Arc<Barrier>,
    source: &'static str,
    expected: CGColor,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        barrier.wait();
        for _ in 0..16 {
            let frame = compile_standalone_svg(source).expect("compile isolated SVG");
            let node = frame.nodes.first().expect("one rectangle");
            let mut paints = node.paints.iter();
            assert_eq!(paints.next().expect("one solid fill").color, expected);
            assert!(paints.next().is_none(), "only one paint");
        }
    })
}
