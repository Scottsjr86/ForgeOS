# FORGEOS-V1-TERMINAL-200 User Guide Source

Status: `CLOSED`

ForgeOS can manage several real interactive terminal sessions at once. Each session
is attached to one exact project and repository, keeps its own raw output and process
identity, and can be independently resized, written to, terminated, and removed.

Terminal working directories must resolve through the registered repository boundary
and fit inside the project's declared roots. Symlink escapes and broader undeclared
working directories are rejected before a process starts. A handle naming the wrong
project or repository cannot operate on the terminal.

Registered build, test, format, and custom command execution is owned by
`FORGEOS-V1-COMMAND-200`. Final terminal rendering, recovery, and session bootstrap
remain later capabilities.
