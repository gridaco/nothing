//! Shared HTML → styled-DOM front-end.
//!
//! One home for the parse-and-cascade sequence that was previously
//! duplicated verbatim between the htmlcss renderer
//! (`collect_styled_tree`) and the HTML importer
//! (`import::html::from_html_str`) — see the seam program,
//! gridaco/nothing#30.
//!
//! Preference toggles are outside this function's signature. They are not
//! caller-isolated: Stylo's static preferences are process-global, so a
//! toggle made before this function affects every later consumer in the
//! process. An explicit cascade environment must replace that ambient state.
//!
//! # Thread safety
//!
//! Uses the process-global DOM slot installed by
//! [`csscascade::adapter::bootstrap_dom`]. Every consumer must currently
//! participate in one process-wide session lock; crate-local locks do not
//! make overlapping sessions safe.

use csscascade::adapter::{self, HtmlDocument};
use csscascade::cascade::CascadeDriver;
use csscascade::dom::DemoDom;
use style::thread_state::{self, ThreadState};

/// Parse HTML and resolve styles via Stylo, returning the styled
/// document handle installed in the global DOM slot.
pub fn parse_and_style(html: &str) -> Result<HtmlDocument, String> {
    // Ensure Stylo thread state is initialized (idempotent after first call).
    thread_state::initialize(ThreadState::LAYOUT);

    // 1. Parse HTML into arena DOM
    let dom =
        DemoDom::parse_from_bytes(html.as_bytes()).map_err(|e| format!("HTML parse error: {e}"))?;

    // 2. Build cascade driver (collects <style> blocks, builds UA + author sheets)
    let mut driver = CascadeDriver::new(&dom);

    // 3. Install DOM into global slot
    let document = adapter::bootstrap_dom(dom);

    // 4. Flush stylist + resolve all styles
    driver.flush(document);
    let _styled_count = driver.style_document(document);

    Ok(document)
}
