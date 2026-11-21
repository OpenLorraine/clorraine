"""Discover local installer regressions; real smoke contexts require explicit invocation."""

from native_review_archive_spec import NativeReviewArchiveShellTests, NativeReviewArchiveTests
from native_review_audit_spec import NativeReviewAuditTests
from native_review_state_spec import NativeReviewCompletionTests, NativeReviewWorkspaceTests


__all__ = [
    "NativeReviewArchiveShellTests", "NativeReviewArchiveTests",
    "NativeReviewAuditTests", "NativeReviewCompletionTests", "NativeReviewWorkspaceTests",
]
"""Only bounded local contracts participate in ordinary unittest discovery."""
