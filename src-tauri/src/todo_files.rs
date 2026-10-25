//! One Markdown file per TODO, beside the database, so an agent told about
//! `#CMUX-12` can read the task itself: its title, status, plan and notes.
//! Rewritten whenever TODOs change; files for deleted TODOs are removed.

use std::path::Path;

use crate::store::Store;
use crate::store::types::TodoRow;

/// `CMUX-12`: how prompts and the UI refer to a TODO.
pub fn todo_ref(number: i64) -> String {
    format!("CMUX-{number}")
}

fn file_name(number: i64) -> String {
    format!("{}.md", todo_ref(number))
}

fn render(todo: &TodoRow) -> String {
    let status = match &todo.done_at {
        Some(at) => format!("done ({at} UTC)"),
        None => "open".to_string(),
    };
    let planned = todo.planned_on.as_deref().unwrap_or("not planned");
    let notes = todo.description.trim();
    format!(
        "# {}: {}\n\n- Status: {status}\n- Planned for: {planned}\n\n## Notes\n\n{}\n",
        todo_ref(todo.number),
        todo.title,
        if notes.is_empty() { "No notes." } else { notes },
    )
}

/// Writes every TODO's file and removes files for TODOs that no longer exist.
pub fn sync_to(dir: &Path, todos: &[TodoRow]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let keep: std::collections::HashSet<String> =
        todos.iter().map(|todo| file_name(todo.number)).collect();
    for todo in todos {
        let path = dir.join(file_name(todo.number));
        let content = render(todo);
        // Unchanged files are left alone, so their timestamps mean something.
        if std::fs::read_to_string(&path).ok().as_deref() != Some(content.as_str()) {
            std::fs::write(&path, content)?;
        }
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("CMUX-") && name.ends_with(".md") && !keep.contains(&name) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}

/// Brings the files up to date with the store. A failure is logged, never fatal:
/// the TODO itself is saved either way.
pub fn sync(store: &Store) {
    let Some(dir) = store.tasks_dir() else {
        return;
    };
    let result = store
        .snapshot()
        .map_err(|error| std::io::Error::other(error.to_string()))
        .and_then(|snapshot| sync_to(&dir, &snapshot.todos));
    if let Err(error) = result {
        log::warn!("could not write TODO files: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo(number: i64, title: &str) -> TodoRow {
        TodoRow {
            id: format!("todo-{number}"),
            number,
            title: title.into(),
            pinned: false,
            description: String::new(),
            planned_on: None,
            done_at: None,
        }
    }

    #[test]
    fn writes_one_file_per_todo_and_removes_deleted_ones() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = todo(1, "Book the venue");
        first.description = "Ask for the big room".into();
        first.planned_on = Some("2026-10-21".into());
        sync_to(dir.path(), &[first.clone(), todo(2, "Ship it")]).unwrap();

        let text = std::fs::read_to_string(dir.path().join("CMUX-1.md")).unwrap();
        assert!(text.starts_with("# CMUX-1: Book the venue\n"));
        assert!(text.contains("- Status: open"));
        assert!(text.contains("- Planned for: 2026-10-21"));
        assert!(text.contains("Ask for the big room"));
        assert!(dir.path().join("CMUX-2.md").exists());

        sync_to(dir.path(), &[first]).unwrap();
        assert!(!dir.path().join("CMUX-2.md").exists());
    }
}
