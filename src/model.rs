use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Named {
    pub gid: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub gid: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Project {
    pub gid: String,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    pub color: Option<String>,
    pub due_on: Option<NaiveDate>,
    pub owner: Option<Named>,
    pub current_status: Option<ProjectStatus>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProjectStatus {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
    pub color: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Membership {
    pub project: Named,
    pub section: Option<Named>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Task {
    pub gid: String,
    pub name: String,
    #[serde(default)]
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub due_on: Option<NaiveDate>,
    pub due_at: Option<DateTime<Utc>>,
    pub assignee: Option<Named>,
    pub assignee_section: Option<Named>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub projects: Vec<Named>,
    #[serde(default)]
    pub memberships: Vec<Membership>,
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub num_subtasks: usize,
    #[serde(default)]
    pub permalink_url: String,
}

impl Task {
    pub fn edit_patch(
        &self,
        name: &str,
        notes: &str,
        date: Option<NaiveDate>,
        assignee: &Option<Named>,
    ) -> Value {
        let mut patch = json!({});
        if name != self.name {
            patch["name"] = json!(name);
        }
        if notes != self.notes {
            patch["notes"] = json!(notes);
        }
        if assignee.as_ref().map(|u| &u.gid) != self.assignee.as_ref().map(|u| &u.gid) {
            patch["assignee"] = json!(assignee.as_ref().map(|u| &u.gid));
        }
        if date != self.due_date() {
            patch["due_on"] = json!(date);
            patch["due_at"] = Value::Null;
        }
        patch
    }

    pub fn due_date(&self) -> Option<NaiveDate> {
        self.due_at
            .map(|d| d.with_timezone(&Local).date_naive())
            .or(self.due_on)
    }

    pub fn overdue(&self, today: NaiveDate) -> bool {
        !self.completed && self.due_date().is_some_and(|date| date < today)
    }

    pub fn activity_key(&self) -> String {
        format!(
            "{}:{}",
            self.gid,
            self.modified_at.map(|d| d.to_rfc3339()).unwrap_or_default()
        )
    }

    pub fn section(&self, project: Option<&str>) -> Named {
        if let Some(project) = project {
            self.memberships
                .iter()
                .find(|m| m.project.gid == project)
                .and_then(|m| m.section.as_ref())
                .cloned()
                .unwrap_or_else(|| Named {
                    gid: "unsectioned".into(),
                    name: "Tasks".into(),
                })
        } else {
            self.assignee_section
                .as_ref()
                .cloned()
                .unwrap_or_else(|| Named {
                    gid: "unsectioned".into(),
                    name: "Recently assigned".into(),
                })
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Story {
    pub gid: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub resource_subtype: String,
    pub created_at: Option<DateTime<Utc>>,
    pub created_by: Option<Named>,
}

#[derive(Clone, Default)]
pub struct WorkspaceData {
    pub workspace: Named,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub users: Vec<Named>,
}

#[derive(Clone, Default)]
pub struct ProjectData {
    pub project: Project,
    pub sections: Vec<Named>,
    pub tasks: Vec<Task>,
}

pub fn update_task_list(tasks: &mut Vec<Task>, task: &Task, belongs: bool) {
    if !belongs {
        tasks.retain(|t| t.gid != task.gid);
    } else if let Some(existing) = tasks.iter_mut().find(|t| t.gid == task.gid) {
        *existing = task.clone();
    } else {
        tasks.push(task.clone());
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TaskFilter {
    #[default]
    Incomplete,
    All,
    Completed,
    Overdue,
}

impl TaskFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::Incomplete => "Incomplete",
            Self::All => "All tasks",
            Self::Completed => "Completed",
            Self::Overdue => "Overdue",
        }
    }
    pub fn matches(self, task: &Task, today: NaiveDate) -> bool {
        match self {
            Self::Incomplete => !task.completed,
            Self::All => true,
            Self::Completed => task.completed,
            Self::Overdue => task.overdue(today),
        }
    }
}

pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|s| s.chars().next())
        .collect::<String>()
        .to_uppercase()
}

pub fn date_label(date: Option<NaiveDate>, today: NaiveDate) -> String {
    match date {
        None => "—".into(),
        Some(d) if d == today => "Today".into(),
        Some(d) if d == today + Duration::days(1) => "Tomorrow".into(),
        Some(d) => d.format("%b %-d").to_string(),
    }
}

pub fn demo() -> (User, WorkspaceData, Vec<ProjectData>) {
    let today = Local::now().date_naive();
    let user = User {
        gid: "demo-me".into(),
        name: "Alex Morgan".into(),
    };
    let me = Named {
        gid: user.gid.clone(),
        name: user.name.clone(),
    };
    let sam = Named {
        gid: "demo-sam".into(),
        name: "Sam Chen".into(),
    };
    let workspace = Named {
        gid: "demo".into(),
        name: "Studio workspace".into(),
    };
    let specs = [
        (
            "demo-launch",
            "Website launch",
            "A thoughtful new home for our product. Coordinate design, development, and launch in one place.",
            "light-blue",
        ),
        (
            "demo-design",
            "Design system",
            "Build a consistent and accessible experience across every screen.",
            "light-purple",
        ),
        (
            "demo-team",
            "Team planning",
            "Make room for focused work and keep the team moving together.",
            "light-green",
        ),
    ];
    let projects: Vec<Project> = specs
        .iter()
        .map(|(gid, name, notes, color)| Project {
            gid: (*gid).into(),
            name: (*name).into(),
            notes: (*notes).into(),
            color: Some((*color).into()),
            due_on: Some(today + Duration::days(14)),
            owner: Some(me.clone()),
            ..Default::default()
        })
        .collect();
    let task_specs = [
        ("Review homepage exploration", 0, 0, Some(0), false),
        ("Finalize the launch checklist", 0, 1, Some(0), false),
        ("Write the product announcement", 0, 0, Some(1), false),
        ("Check responsive layouts", 0, 1, Some(3), false),
        ("Ship the new navigation", 0, 2, Some(-1), true),
        ("Accessibility and keyboard audit", 0, 0, Some(5), false),
        ("Document color and type tokens", 1, 1, Some(-2), false),
        ("Review input and button states", 1, 0, Some(2), false),
        ("Publish the icon library", 1, 2, Some(-1), true),
        ("Plan next week's priorities", 2, 0, Some(4), false),
        ("Share notes from the design review", 2, 2, Some(-1), true),
        ("Collect ideas for the next cycle", 2, 0, None, false),
    ];
    let sections = ["To do", "In progress", "Done"];
    let tasks: Vec<Task> = task_specs.iter().enumerate().map(|(ix,(name,pi,si,offset,completed))| {
        let project = Named { gid: projects[*pi].gid.clone(), name: projects[*pi].name.clone() };
        Task {
            gid: format!("demo-task-{ix}"), name: (*name).into(), completed: *completed,
            completed_at: completed.then(Utc::now), due_on: offset.map(|n| today + Duration::days(n)),
            assignee: Some(if ix == 3 {sam.clone()} else {me.clone()}),
            assignee_section: Some(Named {gid: format!("my-{si}"), name: sections[*si].into()}),
            notes: "Bring the latest work into the review, capture feedback, and agree on the next step.\n\n• Keep the experience simple\n• Check the small details\n• Share a clear handoff".into(),
            projects: vec![project.clone()], memberships: vec![Membership { project, section: Some(Named {gid: format!("section-{pi}-{si}"), name: sections[*si].into()}) }],
            modified_at: Some(Utc::now() - Duration::hours(ix as i64 * 2)), ..Default::default()
        }
    }).collect();
    let pages = projects
        .iter()
        .enumerate()
        .map(|(pi, p)| ProjectData {
            project: p.clone(),
            sections: sections
                .iter()
                .enumerate()
                .map(|(si, s)| Named {
                    gid: format!("section-{pi}-{si}"),
                    name: (*s).into(),
                })
                .collect(),
            tasks: tasks
                .iter()
                .filter(|t| t.projects.iter().any(|p2| p2.gid == p.gid))
                .cloned()
                .collect(),
        })
        .collect();
    (
        user,
        WorkspaceData {
            workspace,
            projects,
            tasks: tasks
                .into_iter()
                .filter(|t| t.assignee.as_ref().is_some_and(|a| a.gid == me.gid))
                .collect(),
            users: vec![me, sam],
        },
        pages,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editing_one_field_preserves_other_server_fields() {
        let task = Task {
            name: "Original".into(),
            notes: "Shared description".into(),
            assignee: Some(Named {
                gid: "123".into(),
                name: "Owner".into(),
            }),
            due_at: Some("2026-09-08T09:30:00Z".parse().unwrap()),
            ..Default::default()
        };
        assert_eq!(
            task.edit_patch("Renamed", &task.notes, task.due_date(), &task.assignee),
            json!({"name":"Renamed"})
        );
        assert_eq!(
            task.edit_patch(&task.name, &task.notes, None, &None),
            json!({"assignee":null,"due_on":null,"due_at":null})
        );
        assert_eq!(
            task.edit_patch(&task.name, &task.notes, task.due_date(), &task.assignee),
            json!({})
        );
    }

    #[test]
    fn task_updates_keep_server_order_and_membership() {
        let mut tasks = vec![
            Task {
                gid: "1".into(),
                ..Default::default()
            },
            Task {
                gid: "2".into(),
                ..Default::default()
            },
        ];
        let task = Task {
            gid: "1".into(),
            name: "Edited".into(),
            ..Default::default()
        };
        update_task_list(&mut tasks, &task, true);
        assert_eq!(tasks[0].name, "Edited");
        assert_eq!(tasks[1].gid, "2");
        update_task_list(&mut tasks, &task, false);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].gid, "2");
    }
    #[test]
    fn filters_dates_sections_and_demo_membership() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let mut task = Task {
            due_on: Some(today - Duration::days(1)),
            ..Default::default()
        };
        assert!(TaskFilter::Overdue.matches(&task, today));
        task.completed = true;
        assert!(!TaskFilter::Overdue.matches(&task, today));
        assert!(TaskFilter::Completed.matches(&task, today));
        assert_eq!(date_label(Some(today), today), "Today");
        assert_eq!(initials("堤 浩一"), "堤浩");
        let (user, data, pages) = demo();
        assert!(
            data.tasks
                .iter()
                .all(|t| t.assignee.as_ref().unwrap().gid == user.gid)
        );
        assert_eq!(pages.iter().map(|p| p.tasks.len()).sum::<usize>(), 12);
    }
}
