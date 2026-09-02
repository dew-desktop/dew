//! What both binaries share.
//!
//! `dew-host` runs applets and `datamodel-surface` reports on what the host
//! implements, and the second must measure the first RATHER THAN A COPY OF IT.
//! While the tool kept its own hand-written list, it reported 35 of 138 for a
//! surface the host had never implemented at all -- the numbers were Aether's,
//! under the host's name, and nothing could have caught it because the two had
//! no code in common.
//!
//! A binary cannot import another binary, so the shared part is a library and
//! both binaries read it. `datamodel::accepts` is the whole point: one function,
//! used by the code path a guest actually takes and by the document that claims
//! what that path does.
pub mod datamodel;
