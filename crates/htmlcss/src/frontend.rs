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
//! Each result owns its DOM and computed style data, so independently styled
//! documents do not share document state. Stylo's static preferences remain
//! process-global; callers that change them must still coordinate those
//! changes.

use csscascade::adapter::DocumentSession;
use csscascade::cascade::CascadeDriver;
use csscascade::dom::DemoDom;
use style::thread_state::{self, ThreadState};

/// Parse HTML and resolve styles via Stylo into an owned document session.
pub fn parse_and_style(html: &str) -> Result<DocumentSession, String> {
    // Ensure Stylo thread state is initialized (idempotent after first call).
    thread_state::initialize(ThreadState::LAYOUT);

    // 1. Parse HTML into arena DOM
    let dom =
        DemoDom::parse_from_bytes(html.as_bytes()).map_err(|e| format!("HTML parse error: {e}"))?;

    // 2. Bind the frozen DOM and its computed style storage to one owner.
    let mut session = DocumentSession::new(dom);

    // 3. Resolve UA + author styles under the session's exclusive borrow.
    let _styled_count = CascadeDriver::new(&mut session).style_document();

    Ok(session)
}
