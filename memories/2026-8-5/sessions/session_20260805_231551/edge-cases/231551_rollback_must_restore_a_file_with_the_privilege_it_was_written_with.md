# Rollback must restore a file with the privilege it was written with

Case:
`Change::FileWritten` and `Change::FileRemoved` carry a `privilege` field, and `Undo::RestoreFile`/`DeleteFile` carry it through to `rollback::service::perform`.

Before that field existed, every file undo ran `sudo install`. For a file in the user's home, such as Steam's `localconfig.vdf`, that prompted for a password on rollback and left the file owned by root, after which Steam silently fails to save its own settings. Caught by rolling back a real write outside the test suite; no unit test covered the privilege of the undo.

Serde defaults the field to `Privilege::Root` for journals written before it existed, because every step that predates it wrote under `/etc`.

Any new step that writes a file must set `privilege` to match the write, not to whatever the neighbouring step uses.
