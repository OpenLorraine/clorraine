//! Specification tests for the raw and conditional store/alias batch.
//!
//! The cases pin the behavior of the raw-deref move, the conditional save, the
//! take-capture stack, the fixed-array alias guard, and the post-store source
//! rules. The batch's specification tasks add the individual ST cases; this
//! file is the placeholder the pair is created with so the test target exists
//! before the implementation starts.
