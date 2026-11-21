# Reviewing Application changes

For changes to TUI interactions, use the corresponding interaction in
`sw-platform/tools/arkedge-cli/src/tui` as the reference. When a user supplies
another example, identify its source and the decision it illustrates.

Before editing, trace one real interaction affected by the change. Follow the
input or driver result through the component that owns the state to the next
command, operation, or redraw. Compare the same path in the reference. Disclose
each intended design difference and its reason in the progress update. A synd
requirement should name an observable behavior, a caller, or a test. Identify
your own design choices explicitly. Use the reference when a difference has
no concrete benefit. Routine implementation choices need no approval.

Review the resulting diff by tracing that interaction again. Account for each
added type, field, helper, and dispatch branch along the path. Check where the
payload moves or gets copied, what wakes the loop, and the order in which
components observe changes. For example, adding a subscriber to a tab change
should require editing the subscriber, without changing the event loop.
Report any unplanned differences or unverified behavior with the result.
