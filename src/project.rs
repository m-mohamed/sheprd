use crate::config::Config;
use crate::error::{Result, SheprdError};
use crate::recipe::Agent;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
}

impl Project {
    pub fn workspace_label(&self, agent: Agent) -> String {
        format!("{}-{agent}", self.name)
    }
}

pub fn resolve(config: &Config, selector: &str) -> Result<Project> {
    let path = PathBuf::from(selector);
    if selector_is_path_like(selector, &path) && path.exists() {
        return project_from_path(&path);
    }

    let matches = discover(config)?
        .into_iter()
        .filter(|project| project.name == selector)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [project] => Ok(project.clone()),
        [] if path.exists() => project_from_path(&path),
        [] => Err(SheprdError::Message(format!(
            "project '{selector}' was not found"
        ))),
        _ => Err(SheprdError::Message(format!(
            "project '{selector}' is ambiguous; pass a path"
        ))),
    }
}

pub fn selector_is_path_like(selector: &str, path: &Path) -> bool {
    path.is_absolute()
        || selector.starts_with('.')
        || selector.contains('/')
        || selector.contains('\\')
}

pub fn discover(config: &Config) -> Result<Vec<Project>> {
    let mut seen = BTreeSet::new();
    let mut projects = Vec::new();
    for configured in &config.projects {
        if !configured.path.exists() {
            continue;
        }
        let path = configured.path.canonicalize()?;
        if seen.insert(path.clone()) {
            projects.push(Project {
                name: configured.name.clone(),
                path,
            });
        }
    }

    for root in &config.roots {
        if !root.exists() {
            continue;
        }
        visit(root, 0, config, &mut seen, &mut projects)?;
    }
    projects.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    Ok(projects)
}

fn visit(
    path: &Path,
    depth: usize,
    config: &Config,
    seen: &mut BTreeSet<PathBuf>,
    projects: &mut Vec<Project>,
) -> Result<()> {
    if depth > config.max_depth || ignored(path, config) {
        return Ok(());
    }

    if path.join(".git").exists() {
        let project = project_from_path(path)?;
        if seen.insert(project.path.clone()) {
            projects.push(project);
        }
        return Ok(());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            visit(&entry.path(), depth + 1, config, seen, projects)?;
        }
    }
    Ok(())
}

fn ignored(path: &Path, config: &Config) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| config.ignore.iter().any(|ignored| ignored == name))
}

fn project_from_path(path: &Path) -> Result<Project> {
    let path = path.canonicalize()?;
    if !path.join(".git").exists() {
        return Err(SheprdError::Message(format!(
            "project path is not a git repository: {}",
            path.display()
        )));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SheprdError::Message("project path has no usable name".into()))?
        .to_string();
    Ok(Project { name, path })
}
