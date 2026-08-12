# A failed CI run is just deleted, not diagnosed into a workflow change

Decision:
Decision: when a CI run fails from a one-off cause (duplicate tag push, transient error), the fix is deleting the failed run - not proposing workflow changes or adding idempotency guards.

Reason: the user asked what went wrong, got the answer, and wanted to delete the run. Proposing a workflow edit turned a 10-second fix into a drawn-out conversation. Match the fix to the problem's actual frequency.
