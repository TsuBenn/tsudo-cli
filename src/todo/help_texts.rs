pub const GENERAL_HELP: &'static str = r#"
tsudo - Simple, Lightweight CLI Todo List

Usage: tsudo [command] [args]...

Command:
    list                   List tasks
    new, add <desc>        Add a new tasks
    complete, done         Complete task(s)
    remove, delete, rm     Remove task(s)
    edit                   Edit task(s)
    config                 Configure current project's Tsudo config
    filter                 List tasks on a specific tag
    detail                 Select and show task(s) details
    journal, log           List changes made to tasks
    help, -h, --help       Show this message

See 'tsudo help <command>' for more information on a specific command
"#;

pub const LIST_HELP: &'static str = r#"
tsudo list

Description:
    List tasks currently in current project

Usage:
    tsudo list [OPTIONS]
    tsudo      [OPTIONS]

Options:
    When no options are provided, tsudo list will behave according to
    the current project's Tsudo config. These options are used to temporary
    override the config's behavior.

    -a, --all        Show all tasks, including hidden ones
    -d, --detail     Show tasks along with their detail if they has one
    -t, --time       Show tasks' creation or completion date and time
    -p, --pending    Show only pending tasks, this override the '-a, -all'
                     behavior

Examples:
    1. List tasks:

        tsudo

    2. List only pending tasks:

        tsudo -p

"#;

pub const NEW_HELP: &'static str = r#"
tsudo new

Description:
    Add a new tasks with a provided description.
    If no description is provided, Tsudo will spin up a inline TUI textfield
    for you to type in the description.
    After description is provided, Tsudo will ask for a tag for the task, the
    inline TUI will suggest existing tags
    if the list has any.

Usage:
    tsudo new [ARGS]
    tsudo add [ARGS]

Examples:
    1. Adding a new tasks of "Buy milk and never come back":

        tsudo add Buy milk and never come back

        or

        tsudo add "Buy milk and never come back"

    2. Giving a task a tag:

        tsudo add Buy milk and never come back
        ? Enter a Tag for the Task: Important

         Important [0/1] (0%):
                   Just now  [ ] Buy milk and never come back

"#;

pub const COMPLETE_HELP: &'static str = r#"
tsudo complete

Description:
    Complete a tasks or multiple tasks using the inline TUI.
    No task id is needed, just type 'tsudo complete' and Tsudo
    will spin up a inline TUI for you to toggle the completion
    status of tasks.

Usage:
    tsudo complete
    tsudo done

Control:
    Basics:
        [i/j/k/l] or the [arrow keys] for "up" "left" "down" and "right"

        [space] to toggle a task's completion status

        [enter] to accept and save the completion status of tasks

    Filter:
        [/] to enter FILTER MODE and fuzzy find your desired task

        [esc] during FILTER MODE to cancel the query, clearing the filter
        and exit FILTER MODE

        [esc] while not in FILTER MODE and while the query is not empty to
        clear the filter

        [enter] during the FILTER MODE to accept the query and you can freely
        move and toggle your tasks

    Advanced Filtering:
        To filter out specific groups, use the "%" at the start of a query
        telling the search engine to search based on group.
        If you want to search for something withing a specific group, use a
        "-" after the group search and search for your desired task within
        that group

        Example:

            %Group name - Task description here

    Note:
        If a tag group is collapsed, press [space] to expand it. Don't press
        [enter] as it will accept, save and exit the program

Examples:
    1. Spin up inline TUI for completion:

        tsudo complete

"#;

pub const REMOVE_HELP: &'static str = r#"
tsudo remove

Description:
    Remove a tasks or multiple tasks using the inline TUI.
    No task id is needed, just type 'tsudo remove' and Tsudo
    will spin up a inline TUI for you to select one or many tasks
    for removal.

Usage:
    tsudo remove
    tsudo delete
    tsudo rm

Control:
    Basics:
        [i/j/k/l] or the [arrow keys] for "up" "left" "down" and "right"

        [space] to select multiple tasks for removal

        [enter] to accept and remove all selected tasks. If no tasks are
        selected, [enter] will remove the one currently being at the cursor

    Filter:
        [/] to enter FILTER MODE and fuzzy find your desired task

        [esc] during FILTER MODE to cancel the query, clearing the filter
        and exit FILTER MODE

        [esc] while not in FILTER MODE and while the query is not empty to
        clear the filter

        [enter] during the FILTER MODE to accept the query and you can freely
        move and select your tasks

    Advanced Filtering:
        To filter out specific groups, use the "%" at the start of a query
        telling the search engine to search based on group.
        If you want to search for something withing a specific group, use a
        "-" after the group search and search for your desired task within
        that group

        Example:

            %Group name - Task description here

    Note:
        If a tag group is collapsed, press [space] will expand it. Press [enter]
        will also expand the collapsed group only when no tasks has been selected.

Examples:
    1. Spin up inline TUI for removal:

        tsudo remove

"#;

pub const EDIT_HELP: &'static str = r#"
tsudo edit

Description:
    Edit a tasks or multiple tasks using the inline TUI.
    No task id is needed, just type 'tsudo edit' and Tsudo
    will spin up a inline TUI for you to select one or many tasks
    for editting.
    If a single task is selected, you can edit its description, detail
    or tag. But if multiple tasks are selected, you can only it their
    tags.

Usage:
    tsudo edit

Control:
    Basics:
        [i/j/k/l] or the [arrow keys] for "up" "left" "down" and "right"

        [space] to select multiple tasks for editing

        [enter] to accept and edit all selected tasks. If no tasks are
        selected, [enter] will select the one currently being at the cursor

    Filter:
        [/] to enter FILTER MODE and fuzzy find your desired task

        [esc] during FILTER MODE to cancel the query, clearing the filter
        and exit FILTER MODE

        [esc] while not in FILTER MODE and while the query is not empty to
        clear the filter

        [enter] during the FILTER MODE to accept the query and you can freely
        move and select your tasks

    Advanced Filtering:
        To filter out specific groups, use the "%" at the start of a query
        telling the search engine to search based on group.
        If you want to search for something withing a specific group, use a
        "-" after the group search and search for your desired task within
        that group

        Example:

            %Group name - Task description here

    Note:
        If a tag group is collapsed, press [space] will expand it. Press [enter]
        will also expand the collapsed group only when no tasks has been selected.

Examples:
    1. Spin up inline TUI for editting:

        tsudo edit

"#;

pub const FILTER_HELP: &'static str = r#"
tsudo filter

Description:
    Behaves like 'tsudo list' but only list tasks of a specific group.
    If no query is provided, Tsudo will spin up an inline TUI showing
    all exising tags for you to select.
    If a query is provided, Tsudo will try its best to match to the closest
    match. If multiple tags matches the query, then Tsudo will let you decide
    via the inline TUI.

Usage:
    tsudo filter [query]

Examples:
    1. Spin up inline TUI for filtering:

        tsudo filter

    2. Listing tasks of a specific tag:

        tsudo filter tag

"#;

pub const DETAIL_HELP: &'static str = r#"
tsudo detail

Description:
    Showing detailed data of a task or multiple tasks.
    No task id is needed, just type 'tsudo remove' and Tsudo
    will spin up a inline TUI for you to select one or many tasks
    to see its detail.

Usage:
    tsudo detail

Control:
    Basics:
        [i/j/k/l] or the [arrow keys] for "up" "left" "down" and "right"

        [space] to select multiple tasks to see their details

        [enter] to accept and list detail of all selected tasks. If no tasks are
        selected, [enter] will select the one currently being at the cursor

    Filter:
        [/] to enter FILTER MODE and fuzzy find your desired task

        [esc] during FILTER MODE to cancel the query, clearing the filter
        and exit FILTER MODE

        [esc] while not in FILTER MODE and while the query is not empty to
        clear the filter

        [enter] during the FILTER MODE to accept the query and you can freely
        move and select your tasks

    Advanced Filtering:
        To filter out specific groups, use the "%" at the start of a query
        telling the search engine to search based on group.
        If you want to search for something withing a specific group, use a
        "-" after the group search and search for your desired task within
        that group

        Example:

            %Group name - Task description here

    Note:
        If a tag group is collapsed, press [space] will expand it. Press [enter]
        will also expand the collapsed group only when no tasks has been selected.

Examples:
    1. Spin up inline TUI for checking details:

        tsudo detail

"#;

pub const CONFIG_HELP: &'static str = r#"
tsudo config

Description:
    Adjust the config of current project's Tsudo config

Usage:
    tsudo config

Control:
    Basics:
        [i/j/k/l] or the [arrow keys] for "up" "left" "down" and "right"

        [enter] to toggle or advance an config option

        [esc] to save the config and exit the program

Examples:
    1. Spin up inline TUI for changing config:

        tsudo config

"#;

pub const JOURNAL_HELP: &'static str = r#"
tsudo journal

Description:
    Show a list of changes made to the todo list in an inline TUI.
    When selected one, Tsudo will show you a snapshot of the specific
    version. You can revert back to that specific version or adjust some
    viewing mode like hiding completed groups, hiding changes or sorting
    tasks by completion status.

Usage:
    tsudo config

Control:
    Basics:
        [i/j/k/l] or the [arrow keys] for "up" "left" "down" and "right"

        [enter] to select an option

        [esc] to accept an option or return back to previous menu

Examples:
    1. Spin up inline TUI for changing config:

        tsudo config

"#;


