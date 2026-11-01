# Tsudo
`tsudo` is a light-weight, project-scoped CLI Task Manager built in Rust.

## Features
- **Project-scoped**: Each project will have its own `.todo` similar to how `.git` works. Preventing bloating the todo list with other project's tasks.
- **Inline TUI**: Instead of passing in tasks ID to complete, remove or edit, `tsudo` provide a light-weight terminal UI to filter, select tasks seamlessly.
- **Only Tags**: Instead of complex groupings and labeling, `tsudo` only have tags, help simple organization and grouping tasks as well as track each group's progress.
- **Compact Listing**: `tsudo` provide some basic features to reduce the amount of tasks shown, prevent distraction from completed task groups.
- **Journaling**: To prevent mistakes from high-speed productions, `tsudo` snapshot every changes of the todo list so developer can revert to previous versions if they made a destructive mistake.

## Installation
### From source
Require Rust's tool chain `cargo` to be installed
```bash
clone https://github.com/TsuBenn/tsudo-cli
cd tsudo-cli
cargo install --path .
```

## Usage
### Listing
Command:
```bash
tsudo list
```
or
```bash
tsudo
```
Description: List tasks currently in current project
