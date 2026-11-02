# tsudo (TsuBenn + todo)
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
### Overview
`tsudo` keeps its commands short and simple while providing an intuitive interface within some commands to reduce frictions when it comes to task management.
```bash
tsudo [command] [args]...
```
| Commands                   | Descriptions                             |
| ---                        | ---                                      |
| `list`                     | List tasks                               |
| `new <desc>`, `add <desc>` | Add a new tasks                          |
| `complete`, `done`         | Complete task(s)                         |
| `remove`, `delete`, `rm`   | Remove task(s)                           |
| `edit`                     | Edit task(s)                             |
| `config`                   | Configure current project's Tsudo config |
| `filter`                   | List tasks on a specific tag             |
| `detail`                   | Select and show task(s) details          |
| `journal`, `log`           | List changes made to tasks               |
| `help`, `-h`, `--help`     | Show this message                        |

### Listing
List tasks currently in current project.
> **Important**
> If no `.todo` directory a present in the current directory or any of the parent directory, no tasks will be printed and `tsudo` will throw a message and no `.todo` directory will be created.
> To initialize a `.todo` directory, check the usage of [adding tasks](#adding-tasks).
>
```bash
tsudo list [options]
# or
tsudo      [options]
```
When no options are provided, `tsudo` list will behave according to the current project's Tsudo config. Some of these options are used to temporary override the config's behavior.

| Options           | Descriptions                                                     |
| ---               | ---                                                              |
| `-a`, `--all`     | Show all tasks, including hidden ones                            |
| `-d`, `--detail`  | Show tasks along with their detail if they has one               |
| `-t`, `--time`    | Show tasks' creation or completion date and time                 |
| `-p`, `--pending` | Show only pending tasks, this override the `-a`, `-all` behavior |

### Adding tasks
Add a new tasks with a provided description.
If no description is provided, `tsudo` will spin up an inline TUI textfield for you to type the description into.
After description is provided, `tsudo` will ask for a tag for the task, the inline TUI will suggest existing tags if the list has any.
> **Important**
> If no `.todo` directory a present in the current directory or any of the parent directory, `tsudo` will ask whether you want to initialize a `.todo` in the current directory.
>
```bash
tsudo add [args]
# or
tsudo new [args]
```
Quotes are option for any thing coming after `add` or `new` as they will be joined together by 1 space.
```
tsudo add Buy milk and never come back
# Is the same as
tsudo add "Buy milk and never come back"
```
