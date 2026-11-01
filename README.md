# Tsudo
**Tsudo** is a light-weight, project-based CLI Task Manager built in Rust.

## Description
The name **Tsudo** is a combination of my name, "TsuBenn", and "Todo".
**Tsudo** focuses on quick and frictionless task management, preventing developers from spending too much time managing tasks instead of being productive.
Instead of managing every single tasks across different projects. **Tsudo** keeps tasks localized to each project in a form of `.todo` directory similar to how `.git` works.
This tool also provide an interactive terminal UI for seamless completing, removing and editting tasks with special features like fuzzy finding, etc.
**Tsudo** also journals the task list changes so developers can revert to previous versions if a mistake was made.

## Installation
From source (Cargo)
```bash
clone https://github.com/TsuBenn/tsudo-cli
cd tsudo-cli
cargo install --path .
```

## Usage
### Listing
Command: `tsudo`

