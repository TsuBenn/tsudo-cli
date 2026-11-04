# tsudo
- `tsudo` is a light-weight, project-scoped CLI Task Manager built in Rust.
- The name `tsudo` comes from the combination of my name `TsuBenn` and `todo`.
- If you find `tsudo` annoying to type, feel free alias it to your liking.

![tsudo demo](assets/demo.gif)

## Features
- **Project-scoped**: Each project will have its own `.todo` similar to how `.git` works. Preventing bloating the todo list with other project's tasks.
- **Inline terminal UI**: Instead of passing in tasks ID to complete, remove or edit, `tsudo` provide a light-weight terminal UI to filter, select tasks seamlessly.
- **Only Tags**: Instead of complex groupings and labeling, `tsudo` only have tags, help simple organizating and grouping tasks as well as track each group's progress.
- **Compact Listing**: `tsudo` provide some basic features to reduce the amount of tasks shown, prevent distraction from completed task groups.
- **Journaling**: To prevent mistakes from a high-speed productions, `tsudo` snapshot every change of the todo list so developer can revert to previous versions if they ever make a destructive mistake.

## Installation
### From source
Require Rust's tool chain `cargo` to be installed
```bash
clone https://github.com/TsuBenn/tsudo-cli
cd tsudo-cli
cargo install --path .
```

Please read the [support](#support) section for more information on whether this program might be supported on your device.

## Usage
### Overview
`tsudo` keeps its commands short and simple while providing an intuitive interface within some commands to reduce frictions when it comes to task management.
```bash
tsudo [command] [args]...
```
| Commands                   | Descriptions                                                     |
| ---                        | ---                                                              |
| `list`                     | List tasks                                                       |
| `new <desc>`, `add <desc>` | Add a new tasks                                                  |
| `complete`, `done`         | Complete task(s)                                                 |
| `remove`, `delete`, `rm`   | Remove task(s)                                                   |
| `edit`                     | Edit task(s)                                                     |
| `config`                   | Configure current project's [Tsudo config](#tsudo-configuration) |
| `filter`                   | List tasks on a specific tag                                     |
| `detail`                   | Select and show task(s) details                                  |
| `journal`, `log`           | List changes made to tasks                                       |
| `undo`                     | Revert to previous change                                        |
| `help`, `-h`, `--help`     | Show this message                                                |

---
### Listing tasks
List tasks currently in current project.
> **Important:**
> If no `.todo` directory is present in the current directory or any of the parent directory, no tasks will be printed and `tsudo` will throw a message and no `.todo` directory will be created.
> To initialize a `.todo` directory, check the usage of [adding tasks](#adding-tasks).
>
```bash
tsudo list [options]
# or
tsudo      [options]
```
When no options are provided, `tsudo` list will behave according to the current project's [Tsudo config](#tsudo-configuration). Some of these options are used to temporary override the config's behavior.

| Options           | Descriptions                                                      |
| ---               | ---                                                               |
| `-a`, `--all`     | Show all tasks, including hidden ones                             |
| `-d`, `--detail`  | Show tasks along with their detail if they has one                |
| `-t`, `--time`    | Show tasks' creation or completion date and time                  |
| `-p`, `--pending` | Show only pending tasks, this override the `-a`, `--all` behavior |

Multiple options can be used, with the exception that `-p` will override the `-a` behavior.

#### Demonstration

<details>
  <summary>Listing tasks demonstrations</summary>
  <h4>Basic listing</h4>
  <img src="assets/listing.gif" alt="Basic listing">
  <h4>Listing with tag <code>-a</code></h4>
  <img src="assets/listing_all.gif" alt="Listing with options `-a`">
  <h4>Listing with tag <code>-d</code></h4>
  <img src="assets/listing_detail.gif" alt="Listing with options `-d`">
  <h4>Listing with tag <code>-t</code></h4>
  <img src="assets/listing_time.gif" alt="Listing with options `-t`">
  <h4>Listing with tag <code>-p</code></h4>
  <img src="assets/listing_pending.gif" alt="Listing with options `-p`">
</details>

---
### Adding tasks
- Add a new tasks with a provided description.
- If no description is provided, `tsudo` will spin up an inline terminal UI textfield for you to type the description into.
- After description is provided, `tsudo` will ask for a tag for the task, the inline terminal UI will suggest existing tags if the list has any. A task's tag can be left blank, though when listed, it would show as `__untagged__:`
- A task can also hold detail, which holds a tasks more descriptive details other than the description itself. `new` or `add` by default does not let you add detail on creation to reduce the step of adding a new task, to add detail to a task, check out [editing](#editing-tasks).
- If no `.todo` directory is present in the current directory or any of the parent directory, `tsudo` will ask whether you want to initialize a `.todo` in the current directory.
```bash
tsudo add [args]
# or
tsudo new [args]
```
Quotes are optional for anything that are after `add` or `new`, as they will be joined together by 1 space.
```bash
tsudo add Buy milk and never come back
# Is the same as
tsudo add "Buy milk and never come back"
```
#### Demonstration

<details>
  <summary>Adding tasks demonstrations</summary>
  <h4>Basic adding</h4>
  <img src="assets/adding.gif" alt="Basic adding">
</details>

---
### Completing tasks
Complete a task or multiple tasks.
```bash
tsudo complete
# or
tsudo done
```
- After accepting the completion status, `tsudo` will print out the whole list, highlighting tasks that have their completion status changed.

- For more information on how to navigate the interface, check the [control](#controls) section

#### Demonstration

<details>
  <summary>Completing tasks demonstrations</summary>
  <h4>Basic completion</h4>
  <img src="assets/completing.gif" alt="Basic completion">
</details>

---
### Removing tasks
Remove a task or multiple tasks.
```bash
tsudo remove
# or
tsudo delete
# or
tsudo rm
```
- After accepting the tasks for removal, `tsudo` while print out the whole list, highlighting tasks that has been removed.

- For more information on how to navigate the interface, check the [control](#controls) section

#### Demonstration

<details>
  <summary>Removing tasks demonstrations</summary>
  <h4>Singular removal</h4>
  <img src="assets/removing.gif" alt="Singular removal">
  <h4>Multiple removal</h4>
  <img src="assets/removing_multiple.gif" alt="Multiple removal">
</details>

---
### Editing tasks
Edit a task or multiple tasks.
```bash
tsudo edit
```
- `tsudo` lets you choose one or many tasks for editing.
- After selecting a task, you can choose to edit a task's description, detail or tag.
- If multiple tasks are selected, you can only choose to edit their tags.
- A detail of a task is empty on creation by default to reduce the steps of adding a new task. But sometimes, you might want to add more to a task than just its description, so to add detail to a task, `edit` is the way.

- For more information on how to navigate the interface, check the [control](#controls) section

#### Demonstration

<details>
  <summary>Editing tasks demonstrations</summary>
  <h4>Singular editing</h4>
  <img src="assets/editing.gif" alt="Singular editing">
  <h4>Multiple editing</h4>
  <img src="assets/editing_multiple.gif" alt="Multiple editing">
</details>

---
### Filtering tasks
List tasks of a specific tag.
```bash
tsudo filter [querys]
```
- If query is empty, `tsudo` will prompt you a list of tags for you to choose.
- If a query is provided, `tsudo` will try its best to find the matching tag and list out the tag's tasks. Else if more than one tags are matched, `tsudo` will prompt you to choose which tag you meant to filter.
- If no tags matches the query whatsoever, the `tsudo` will behave similar to when you don't provide any query at all.
- After accepting a tag or multiple tags, `tsudo` behaves like [listing](#listing-tasks) but only printing out the tasks of the tags you've chosen.

#### Demonstration

<details>
  <summary>Filtering tag group demonstrations</summary>
  <h4>Basic filtering</h4>
  <img src="assets/filter.gif" alt="Basic filtering">
  <h4>Filter with query</h4>
  <img src="assets/filter_query.gif" alt="Filter with query">
  <h4>Filter with query and matching</h4>
  <img src="assets/filter_query_match.gif" alt="Filter with query and matching">
</details>

---
### Tasks details
Check a tasks detail information.
```bash
tsudo detail
```
- This is different from [listing](#listing-tasks)'s `-d` option.
- While the `-d` option from [listing](#listing-tasks) list out all of the tasks along with their details shown, this command lets you choose which task to show the detail of and print out a more detailed details than the `-d` option from [listing](#listing-tasks).
- `tsudo` lets you choose one or many tasks to show the detail of.

- For more information on how to navigate the interface, check the [control](#controls) section

#### Demonstration

<details>
  <summary>Listing details of tasks demonstrations</summary>
  <h4>Singular detail listing</h4>
  <img src="assets/detail.gif" alt="Singular detail listing">
  <h4>Multiple detail listing</h4>
  <img src="assets/detail_multiple.gif" alt="Multiple detail listing">
</details>

---
### `tsudo` Configuration
Configure how `tsudo` behaves.
```bash
tsudo config
```
- A `tsudo` config is not global, meaning that `tsudo` config can be varied across projects.

#### Show Creation/Completion Time of Tasks
- **Possible value:** `true`/`false`
- Show tasks creation date and time when [listing](#listing-tasks). When a task is completed, its completion date and time will be shown instead.
- This option can be temporary overriden to `true` by using the `-t` option when using [listing](#listing-tasks).
#### Show Progress of Task Groups
- **Possible value:** `true`/`false`
- Show a task group's progress by how many tasks out of all tasks in that group has been completed as well as a percentage of that tag group's completion
#### Show Completed Task Range
- **Possible value:** `Inf`/`0h`/`6h`/`12h`/`24h`/`48h`
- Instead of showing all tasks everytime, which might lead to the bloating of the list when [listing](#listing-tasks), this option will hide any completed tasks that has been completed for more than the specified hour.
- This option can be temporary overriden to `true` by using the `-a` or `-p` option when using [listing](#listing-tasks).
#### Hide Tasks in Fully Completed Groups
- **Possible value:** `true`/`false`
- While the previous option hide those that has been completed for a while, this will hide any tag groups that has the completion status of 100%, help debloating the [listing](#listing-tasks).
- This option can be temporary overriden to `true` by using the `-a` option when using [listing](#listing-tasks).> **Important:**
> Having this option turned on means the behavior of the selection interface will also be changed. Check the [Control](#controls) -> [Collapsed Group](#collapsed-group) section on how this changes the behavior of the list selection.
#### Sort Tasks by Completion
- **Possible value:** `true`/`false`
- Push any pending tasks to the top when [listing](#listing-tasks), [completing](#completing-tasks), [removing](#removing-tasks) or [editing](#editing-tasks).
- This will not affect the original order of the todo list.

- For more information on how to navigate the interface, check the [control](#controls) section

#### Demonstration

<details>
  <summary>Config demonstrations</summary>
  <h4>Show Creation/Completion Time of Tasks</h4>
  <img src="assets/show_time.gif" alt="Show Creation/Completion Time of Tasks">
  <h4>Show Progress of Task Groups</h4>
  <img src="assets/show_progress.gif" alt="Show Progress of Task Groups">
  <h4>Show Completed Task Range</h4>
  <img src="assets/completion_range.gif" alt="Show Completed Task Range">
  <h4>Hide Tasks in Fully Completed Group</h4>
  <img src="assets/hide_completed.gif" alt="Hide Tasks in Fully Completed Group">
  <h4>Sort Tasks by Completion</h4>
  <img src="assets/sort_completion.gif" alt="Sort Tasks by Completion">
</details>

---
### Journaling Changes
List changes made to the todo list.
```bash
tsudo journal
# or
tsudo log
```
- This will list out changes that has been made to the todo list. When selecting a version, it will print out the snapshot of the todo list of that version.
- You can decide to whether revert to that version or not.
- The files provided by the journaling feature are ignored by git by default, for git to track the snapshots and journal, remove the `.gitignore` in `.todo`.
> **Important:**
> As of right now, there's no way you can clean up the snapshots and it will eventually take up quite a lot of space if your list is very long and you make a lot of changes. To clean up, you can always delete files within the `.todo/history` directory. Though you might still see all of the version when running `tsudo journal`, that's because `.todo/journal.jsonl` is the one tracking the changes and points `tsudo` to the right `*.json` within `.todo/history` for you to view or revert to. You don't have to change anything in the `.todo/journal.jsonl` if you don't want to as what important are your recent changes and selecing a version that points to nothing won't hurt anyone either.

- For more information on how to navigate the interface, check the [control](#controls) section

#### Demonstration

<details>
  <summary>Journal demonstrations</summary>
  <h4>Using Journal to revert Removal</h4>
  <img src="assets/journal.gif" alt="Using Journal to revert Removal">
</details>

### Undoing Changes
Revert to the previous change.
```bash
tsudo undo
```
> `undo` is an automated version of `journal`.
> Instead of going through the [journal](#journaling-changes) list just to revert 1 version back, `undo` would automatically do that for you.

---
### Controls
Instead of you entering a task ID for [completing](#completing-tasks), [removing](#removing-tasks) or [editing](#editing-tasks), `tsudo` provide a interactive inline terminal UI for a more intuitive input and selections.
#### Basics
- `i/j/k/l` or the `arrow keys` for "up", "down", "previous page" and "next page".
> If you're complaining about me not using `h/j/k/l` then I'm sorry, I find `i/j/k/l` easier to use and my Neovim/Helix config uses this bindings for movement so there's that.
- `esc` to cancel the prompt, or in some cases, return to previous menu.
- For `enter` and `space`, they behave differently depends on the mode, but overall, `enter` usually means to accept, and `space` usually means to toggle. Check the next few sections for more information on how `enter` and `space` behave in different modes.
- In multi-selection mode, `Ctrl+A` will select all, `Ctrl+D` will deselect all, and `Ctrl+Z` will revert the beginning state.
> **Note:** When using filter, the select all/deselect all will only be made on the result tasks.
##### [Complete](#completing-tasks):
- `enter` means accept completion status.
- `space` to toggle a task's completion status.
##### [Remove](#removing-tasks):
- `enter` if no task has been selected means to remove the task at the cursor.
- `enter` if one or more tasks has been selected means to accept the selected tasks for removal.
- `space` to select or deselect a task for removal.
##### [Edit](#editing-tasks):
- `enter` if no task has been selected means to edit the task at the cursor.
- `enter` if one or more tasks has been selected means to accept the selected tasks for editing. And like the [editing](#editing-tasks) section mentioned, when multiple tasks are selected, the description and detail editing section will be crossed out and you can only edit their tags.
- `space` to select or deselect a task for editing.
##### [Config](#tsudo-configuration):
- `enter` to advance or toggle a config option.
- `space` does nothing, unfortunately.
- `esc` to accept changes of the config and exit the program.
##### [Journal](#journaling-changes):
- `enter` to select a version of changes.
##### Collapsed Group:
- If you have activated the [Hide Tasks in Fully Completed Group](#hide-tasks-in-fully-completed-groups) in the [config](#tsudo_configuration), groups within the task selection interface would colapse the options within the completed groups and will show something like `[-] [8 collapsed items]`.
- To expand the collapsed groups, you would want to use `space`. You can go back by `esc`, or continue to search within the group by `/` and type.
- When in *FILTER MODE*, the collapsed groups will be automatically expanded out for searching.
<details>
  <summary>Demonstrations</summary>
  <h4>Collapsed groups behavior</h4>
  <img src="assets/collapsed_groups.gif" alt="Collapsed groups behavior">
</details>

#### Filter:
- `/` to enter *FILTER MODE* and fuzzy find your desired task.
- `esc` during *FILTER MODE* to cancel the query, clearing the filter and exit *FILTER MODE*.
- While not in *FILTER MODE* and the query is also not empty, `esc` while clear the filter and does not cancel the prompt.
- `enter` during the FILTER MODE to accept the query and you can freely move, select or toggle your tasks.
<details>
  <summary>Demonstrations</summary>
  <h4>Basic filtering during seleciton</h4>
  <img src="assets/select_filter.gif" alt="Basic filtering during seleciton">
</details>

#### Advanced Filtering:
- To filter out specific groups, type a `%` at the start of a query telling the search engine to search based on group.
- If you want to search for something within a specific group, use a "-" after the group search and search for your desired task within that group
Example:
```
%Group name - Task description here
```
<details>
  <summary>Demonstrations</summary>
  <h4>Filtering groups during selection</h4>
  <img src="assets/select_filter_group.gif" alt="Filtering groups during selection">
</details>

## Support
As of right now, the only machine that this program works is on my Arch Linux Desktop and I'm using the Ghostty Terminal. I tried using this program on my Phone's Termux and the coloring is rather broken so I'm not guaranteed that every terminal would work as intended. Some terminal might experience flickers as my code sucks and it clears the screen every time it updates the screen to there's that.

## Contributing
Uh... I doubt anyone would even use this enough to want to contribute?

## Authors
Just me (TsuBenn)

## License
[MIT](https://choosealicense.com/licenses/mit/)

## Project status
This is a side project that I made to serve one of my main project, so it either gets update as I use it more or this is all I have to offer.
