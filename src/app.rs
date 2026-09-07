mod task_detail;
mod views;

use crate::model::Task;
use crate::{
    api::AsanaClient,
    model::*,
    oauth::{self, PendingAuth, Session},
    preferences::Preferences,
};
use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, Utc};
use gpui_kit::{
    component::{
        input::{InputEvent, InputState, TextareaState},
        *,
    },
    *,
};
use serde_json::json;
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

actions!(
    asana_gpui,
    [Quit, Refresh, SearchTasks, NewTask, CloseDetail, SaveTask]
);

#[derive(Clone, PartialEq, Eq)]
enum Page {
    Home,
    Inbox,
    MyTasks,
    Projects,
    Project(String),
    Settings,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Overview,
    List,
    Board,
    Calendar,
    Dashboard,
}

pub struct WorkspaceApp {
    focus: FocusHandle,
    api: Option<AsanaClient>,
    user: Option<User>,
    workspaces: Vec<Named>,
    data: WorkspaceData,
    project: Option<ProjectData>,
    demo_pages: Vec<ProjectData>,
    demo: bool,
    prefs: Preferences,
    preferences_writable: bool,
    page: Page,
    view: View,
    filter: TaskFilter,
    home_filter: TaskFilter,
    sort_by_due: bool,
    collapsed: HashSet<String>,
    sidebar_visible: bool,
    sidebar_layout: Entity<resizable::ResizableState>,
    detail_layout: Entity<resizable::ResizableState>,
    inbox_archive: bool,
    inbox_unread: bool,
    query: Entity<InputState>,
    draft: Entity<InputState>,
    project_draft: Entity<InputState>,
    notepad: Entity<TextareaState>,
    selected: Option<Task>,
    title: Entity<InputState>,
    description: Entity<TextareaState>,
    due: Entity<InputState>,
    assignee: Option<Named>,
    comment: Entity<TextareaState>,
    stories: Vec<Story>,
    subtasks: Vec<Task>,
    client_id: Entity<InputState>,
    client_secret: Entity<InputState>,
    redirect: Entity<InputState>,
    scopes: Entity<InputState>,
    auth_code: Entity<InputState>,
    pending_auth: Option<PendingAuth>,
    cancel_auth: Option<Arc<AtomicBool>>,
    busy: Option<String>,
    request_id: u64,
    credential_generation: u64,
    credential_write: Option<gpui_kit::Task<()>>,
    error: Option<String>,
    status: String,
    calendar_month: NaiveDate,
    _subscriptions: Vec<Subscription>,
}

impl WorkspaceApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let query = input("Search tasks and projects…", window, cx);
        let draft = input("What needs to be done?", window, cx);
        let title = input("Task name", window, cx);
        let due = input("YYYY-MM-DD", window, cx);
        let description =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Add a description…"));
        let comment = cx.new(|cx| TextareaState::new(window, cx).placeholder("Write a comment…"));
        let notepad = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Capture a thought, a reminder, or your next idea…")
        });
        let mut subscriptions = Vec::new();
        let sidebar_layout = cx.new(|_| resizable::ResizableState::default());
        let detail_layout = cx.new(|_| resizable::ResizableState::default());
        for layout in [&sidebar_layout, &detail_layout] {
            subscriptions.push(cx.observe(layout, |_, _, cx| cx.notify()));
        }
        subscriptions.push(cx.on_app_quit(|this, cx| {
            let previous = this.credential_write.take();
            let keep_alive = cx.entity();
            async move {
                if let Some(previous) = previous {
                    previous.await;
                }
                drop(keep_alive);
            }
        }));
        for state in [&query, &title, &due, &draft] {
            subscriptions.push(cx.subscribe(state, |_, _, _: &InputEvent, cx| cx.notify()));
        }
        for state in [&description, &comment, &notepad] {
            subscriptions.push(cx.subscribe(state, |_, _, _: &InputEvent, cx| cx.notify()));
        }
        subscriptions.push(
            cx.subscribe_in(&draft, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.create_task(window, cx);
                }
            }),
        );
        let client_id = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Client ID")
                .default_value(std::env::var("ASANA_CLIENT_ID").unwrap_or_default())
        });
        let client_secret = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Client secret")
                .masked(true)
                .default_value(std::env::var("ASANA_CLIENT_SECRET").unwrap_or_default())
        });
        let mut app = Self {
            focus: cx.focus_handle(),
            api: None,
            user: None,
            workspaces: Vec::new(),
            data: WorkspaceData::default(),
            project: None,
            demo_pages: Vec::new(),
            demo: false,
            prefs: Preferences::default(),
            preferences_writable: true,
            page: Page::Home,
            view: View::List,
            filter: TaskFilter::Incomplete,
            home_filter: TaskFilter::Incomplete,
            sort_by_due: false,
            collapsed: HashSet::new(),
            sidebar_visible: true,
            sidebar_layout,
            detail_layout,
            inbox_archive: false,
            inbox_unread: false,
            query,
            draft,
            title,
            due,
            description,
            comment,
            notepad,
            project_draft: input("New project name", window, cx),
            selected: None,
            assignee: None,
            stories: Vec::new(),
            subtasks: Vec::new(),
            client_id,
            client_secret,
            redirect: cx.new(|cx| {
                InputState::new(window, cx).default_value(
                    std::env::var("ASANA_REDIRECT_URI")
                        .unwrap_or_else(|_| oauth::DEFAULT_REDIRECT.into()),
                )
            }),
            scopes: cx.new(|cx| {
                InputState::new(window, cx).default_value(
                    std::env::var("ASANA_OAUTH_SCOPES").unwrap_or_else(|_| "default".into()),
                )
            }),
            auth_code: input("Authorization code", window, cx),
            pending_auth: None,
            cancel_auth: None,
            busy: None,
            request_id: 0,
            credential_generation: 0,
            credential_write: None,
            error: None,
            status: String::new(),
            calendar_month: Local::now().date_naive().with_day(1).unwrap(),
            _subscriptions: subscriptions,
        };
        if std::env::args().any(|a| a == "--demo") {
            app.enter_demo(window, cx);
        } else if std::env::args().any(|a| a == "--pat") {
            app.connect_pat(window, cx);
        } else {
            app.busy = Some("Opening your workspace".into());
            let credentials = cx.read_credentials(oauth::CREDENTIAL_SERVICE);
            cx.spawn_in(window,async move |view,cx| {
                let result=credentials.await;
                let _=view.update_in(cx,|this,window,cx| {
                    this.busy=None;
                    match result {
                        Ok(Some((_,bytes)))=>match serde_json::from_slice::<Session>(&bytes).map_err(anyhow::Error::from).and_then(AsanaClient::new) {
                            Ok(api)=>this.load_identity(api,true,window,cx),
                            Err(_)=>this.error=Some("The saved session could not be opened. Connect again.".into()),
                        },
                        Ok(None)=>{},
                        Err(_)=>this.error=Some("Could not read the system credential store. You can still connect for this session.".into()),
                    }
                    cx.notify();
                });
            }).detach();
        }
        app.focus.focus(window, cx);
        app
    }

    fn work<R: Send + 'static>(
        &mut self,
        label: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        operation: impl FnOnce() -> Result<R> + Send + 'static,
        complete: impl FnOnce(&mut Self, R, &mut Window, &mut Context<Self>) + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(label.into());
        self.error = None;
        self.request_id += 1;
        let request_id = self.request_id;
        let job = cx.background_executor().spawn(async move { operation() });
        cx.spawn_in(window, async move |view, cx| {
            let result = job.await;
            let _ = view.update_in(cx, |this, window, cx| {
                if this.request_id != request_id {
                    return;
                }
                this.busy = None;
                match result {
                    Ok(result) => complete(this, result, window, cx),
                    Err(error) => this.error = Some(error.to_string()),
                }
                this.persist_credentials(cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn persist_credentials(&mut self, cx: &mut Context<Self>) {
        if let Some(api) = self.api.clone() {
            self.persist_client(&api, cx);
        }
    }

    fn persist_client(&mut self, api: &AsanaClient, cx: &mut Context<Self>) {
        if !api.is_oauth() {
            return;
        }
        let Ok(bytes) = api.credentials() else { return };
        let previous = self.credential_write.take();
        let generation = self.credential_generation;
        self.credential_write = Some(cx.spawn(async move |view, cx| {
            if let Some(previous) = previous { previous.await; }
            if !view.update(cx, |this, _| this.credential_generation == generation).unwrap_or(false) { return; }
            let save = cx.update(|cx| cx.write_credentials(oauth::CREDENTIAL_SERVICE, "session", &bytes));
            if save.await.is_err() {
                let _ = view.update(cx, |this, cx| {
                    if this.credential_generation == generation {
                        this.error = Some("Connected for this session, but the system credential store could not save it.".into());
                        cx.notify();
                    }
                });
            }
        }));
    }

    fn load_identity(
        &mut self,
        api: AsanaClient,
        restoring: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.work(
            "Connecting to Asana",
            window,
            cx,
            move || {
                let result = (|| -> Result<_> {
                    let (user, workspaces) = api.identity()?;
                    let preferences = Preferences::load(&user.gid);
                    let preferred = std::env::var("ASANA_WORKSPACE_ID").unwrap_or_else(|_| {
                        preferences
                            .as_ref()
                            .map(|p| p.workspace.clone())
                            .unwrap_or_default()
                    });
                    let workspace = workspaces
                        .iter()
                        .find(|w| w.gid == preferred)
                        .or(workspaces.first())
                        .cloned()
                        .ok_or_else(|| {
                            anyhow::anyhow!("No Asana workspace is available for this account.")
                        })?;
                    let data = api.workspace(workspace)?;
                    Ok((user, workspaces, preferences, data))
                })();
                Ok((api, result))
            },
            move |this, (api, result), window, cx| {
                let (user, workspaces, preferences, data) = match result {
                    Ok(result) => result,
                    Err(error) => {
                        if restoring {
                            this.persist_client(&api, cx);
                        }
                        this.error = Some(error.to_string());
                        return;
                    }
                };
                this.credential_generation += 1;
                this.api = Some(api);
                this.demo = false;
                this.user = Some(user);
                this.workspaces = workspaces;
                this.data = data;
                this.project = None;
                this.selected = None;
                this.page = Page::Home;
                this.demo_pages.clear();
                this.stories.clear();
                this.subtasks.clear();
                this.draft.update(cx, |s, cx| s.set_value("", window, cx));
                this.project_draft
                    .update(cx, |s, cx| s.set_value("", window, cx));
                this.query.update(cx, |s, cx| s.set_value("", window, cx));
                match preferences {
                    Ok(prefs) => {
                        this.prefs = prefs;
                        this.preferences_writable = true;
                    }
                    Err(error) => {
                        this.prefs = Preferences::default();
                        this.error = Some(error.to_string());
                        this.preferences_writable = false;
                    }
                }
                this.prefs.workspace = this.data.workspace.gid.clone();
                this.notepad.update(cx, |s, cx| {
                    s.set_value(this.prefs.notes.clone(), window, cx)
                });
                crate::theme::apply(this.prefs.light_mode, cx);
                this.save_preferences();
                this.status = "Up to date".into();
            },
        );
    }

    fn load_workspace(&mut self, workspace: Named, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        let Some(api) = self.api.clone() else { return };
        self.work(
            "Loading workspace",
            window,
            cx,
            move || api.workspace(workspace),
            |this, data, window, cx| {
                this.prefs.workspace = data.workspace.gid.clone();
                this.data = data;
                this.project = None;
                this.selected = None;
                this.page = Page::Home;
                this.query.update(cx, |s, cx| s.set_value("", window, cx));
                this.save_preferences();
                this.status = "Up to date".into();
            },
        );
    }

    fn connect_pat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) || self.pending_auth.is_some() {
            return;
        }
        match Session::from_environment_token().and_then(AsanaClient::new) {
            Ok(api) => {
                self.load_identity(api, false, window, cx);
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    fn connect_oauth(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) || self.pending_auth.is_some() {
            return;
        }
        let pending = PendingAuth::new(
            self.client_id.read(cx).value().to_string(),
            self.client_secret.read(cx).value().to_string(),
            self.redirect.read(cx).value().to_string(),
            self.scopes.read(cx).value().to_string(),
        );
        match pending {
            Ok(pending) => {
                self.cancel_auth = Some(pending.cancelled.clone());
                cx.open_url(&pending.authorize_url);
                if pending.is_manual() {
                    self.pending_auth = Some(pending);
                    self.status = "Authorize in your browser, then paste the code below.".into();
                } else {
                    self.work(
                        "Waiting for Asana authorization",
                        window,
                        cx,
                        move || pending.wait(),
                        |this, session, window, cx| this.accept_session(session, window, cx),
                    );
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn accept_session(&mut self, session: Session, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_auth = None;
        self.pending_auth = None;
        self.auth_code
            .update(cx, |s, cx| s.set_value("", window, cx));
        match AsanaClient::new(session) {
            Ok(api) => {
                self.load_identity(api, false, window, cx);
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn finish_manual(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let code = self.auth_code.read(cx).value().to_string();
        if code.trim().is_empty() {
            self.error = Some("Paste the authorization code first.".into());
            cx.notify();
            return;
        }
        if let Some(pending) = self.pending_auth.take() {
            self.work(
                "Completing sign-in",
                window,
                cx,
                move || pending.exchange(&code),
                |this, session, window, cx| this.accept_session(session, window, cx),
            );
        }
    }

    fn cancel_signin(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.cancel_auth.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.pending_auth = None;
        self.request_id += 1;
        self.busy = None;
        self.status = "Sign-in cancelled".into();
        cx.notify();
    }

    fn enter_demo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (user, data, pages) = crate::model::demo();
        self.user = Some(user);
        self.workspaces = vec![data.workspace.clone()];
        self.data = data;
        self.demo_pages = pages;
        self.demo = true;
        self.api = None;
        self.page = Page::Home;
        self.selected = None;
        self.project = None;
        self.prefs = Preferences::default();
        self.busy = None;
        self.error = None;
        self.notepad.update(cx,|s,cx|s.set_value("One place for today's work.\n\nChoose a task to see its details, or explore a project in List, Board, and Calendar.",window,cx));
        cx.notify();
    }

    fn save_preferences(&mut self) -> bool {
        if self.demo {
            return true;
        }
        if !self.preferences_writable {
            return false;
        }
        if let Some(user) = &self.user
            && let Err(error) = self.prefs.save(&user.gid)
        {
            self.error = Some(error.to_string());
            return false;
        }
        true
    }

    fn dirty(&self, cx: &App) -> bool {
        self.selected.as_ref().is_some_and(|task| {
            self.title.read(cx).value().as_ref() != task.name
                || self.description.read(cx).value().as_ref() != task.notes
                || self.due.read(cx).value().as_ref()
                    != task.due_date().map(|d| d.to_string()).unwrap_or_default()
                || self.assignee != task.assignee
                || !self.comment.read(cx).value().trim().is_empty()
        })
    }

    pub(crate) fn can_leave(&mut self, cx: &mut Context<Self>) -> bool {
        if self.busy.is_some() {
            return false;
        }
        if self.dirty(cx) {
            self.error=Some("You have unsaved task changes or a comment draft. Save, post, or discard them before leaving this task.".into());
            cx.notify();
            return false;
        }
        if !self.draft.read(cx).value().trim().is_empty()
            || !self.project_draft.read(cx).value().trim().is_empty()
        {
            self.error = Some(
                "Create or clear the new task or project draft before leaving this page.".into(),
            );
            cx.notify();
            return false;
        }
        let notes = self.notepad.read(cx).value().to_string();
        if self.user.is_some() && notes != self.prefs.notes {
            let previous = std::mem::replace(&mut self.prefs.notes, notes);
            if !self.save_preferences() {
                self.prefs.notes = previous;
                self.error = Some("The private note could not be saved. Copy it or restore its saved content before leaving.".into());
                cx.notify();
                return false;
            }
        }
        true
    }

    pub(crate) fn quit(&mut self, cx: &mut Context<Self>) {
        if self.cancel_auth.is_some() {
            self.cancel_signin(cx);
        }
        if !self.can_leave(cx) {
            return;
        }
        self.busy = Some("Finishing session storage".into());
        let previous = self.credential_write.take();
        cx.spawn(async move |_, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            cx.update(|cx| cx.quit());
        })
        .detach();
        cx.notify();
    }

    fn navigate(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        self.selected = None;
        self.collapsed.clear();
        self.error = None;
        self.query.update(cx, |s, cx| s.set_value("", window, cx));
        if let Page::Project(gid) = &page {
            let gid = gid.clone();
            if self.demo {
                self.project = self
                    .demo_pages
                    .iter()
                    .find(|p| p.project.gid == gid)
                    .cloned();
                self.page = page;
                self.view = View::List;
            } else if let Some(api) = self.api.clone() {
                self.work(
                    "Loading project",
                    window,
                    cx,
                    move || api.project(&gid),
                    move |this, project, _, _| {
                        this.project = Some(project);
                        this.page = page;
                        this.view = View::List;
                    },
                );
            }
        } else {
            self.page = page;
            self.view = View::List;
        }
        cx.notify();
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        if self.demo {
            self.status = "Demo data is stored in memory".into();
            cx.notify();
            return;
        }
        let Some(api) = self.api.clone() else { return };
        let workspace = self.data.workspace.clone();
        let project_gid = match &self.page {
            Page::Project(gid) => Some(gid.clone()),
            _ => None,
        };
        let selected_gid = self.selected.as_ref().map(|t| t.gid.clone());
        self.work(
            "Refreshing",
            window,
            cx,
            move || {
                Ok((
                    api.workspace(workspace)?,
                    project_gid.map(|gid| api.project(&gid)).transpose()?,
                    selected_gid
                        .map(|gid| -> Result<_> {
                            Ok((api.task(&gid)?, api.stories(&gid)?, api.subtasks(&gid)?))
                        })
                        .transpose()?,
                ))
            },
            |this, (data, project, selected), window, cx| {
                this.data = data;
                if project.is_some() {
                    this.project = project;
                }
                if let Some((task, stories, subtasks)) = selected {
                    this.set_editor(task, window, cx);
                    this.stories = stories;
                    this.subtasks = subtasks;
                }
                this.status = format!("Updated {}", Local::now().format("%H:%M"));
            },
        );
    }

    fn open_task(&mut self, task: Task, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        self.prefs.read.insert(task.activity_key());
        self.save_preferences();
        self.stories.clear();
        self.subtasks.clear();
        self.set_editor(task.clone(), window, cx);
        if self.demo {
            self.stories=vec![Story {gid:format!("story-{}",task.gid),text:"The latest version is ready to review. Let's capture feedback here and keep the next step clear.".into(),resource_subtype:"comment_added".into(),created_at:task.modified_at,created_by:Some(Named{gid:"demo-sam".into(),name:"Sam Chen".into()})}];
        } else if let Some(api) = self.api.clone() {
            let gid = task.gid;
            self.work(
                "Loading task details",
                window,
                cx,
                move || Ok((api.task(&gid)?, api.stories(&gid)?, api.subtasks(&gid)?)),
                |this, (task, stories, subtasks), window, cx| {
                    this.set_editor(task, window, cx);
                    this.stories = stories;
                    this.subtasks = subtasks;
                },
            );
        }
        cx.notify();
    }

    fn set_editor(&mut self, task: Task, window: &mut Window, cx: &mut Context<Self>) {
        self.title
            .update(cx, |s, cx| s.set_value(task.name.clone(), window, cx));
        self.description
            .update(cx, |s, cx| s.set_value(task.notes.clone(), window, cx));
        self.due.update(cx, |s, cx| {
            s.set_value(
                task.due_date().map(|d| d.to_string()).unwrap_or_default(),
                window,
                cx,
            )
        });
        self.comment.update(cx, |s, cx| s.set_value("", window, cx));
        self.assignee = task.assignee.clone();
        self.selected = Some(task);
    }

    fn upsert(&mut self, task: Task) {
        let assigned = self
            .user
            .as_ref()
            .is_some_and(|u| task.assignee.as_ref().is_some_and(|a| a.gid == u.gid));
        update_task_list(&mut self.data.tasks, &task, assigned);
        for project in self.demo_pages.iter_mut().chain(self.project.iter_mut()) {
            let belongs = task.projects.iter().any(|p| p.gid == project.project.gid);
            update_task_list(&mut project.tasks, &task, belongs);
        }
        if self.selected.as_ref().is_some_and(|t| t.gid == task.gid) {
            self.selected = Some(task);
        }
    }

    fn apply_clean_update(&mut self, task: Task, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.as_ref().is_some_and(|t| t.gid == task.gid) {
            self.set_editor(task.clone(), window, cx);
        }
        self.upsert(task);
    }

    fn toggle_task(&mut self, mut task: Task, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        if self.demo {
            task.completed = !task.completed;
            task.completed_at = task.completed.then(Utc::now);
            task.modified_at = Some(Utc::now());
            self.apply_clean_update(task, window, cx);
            cx.notify();
        } else if let Some(api) = self.api.clone() {
            self.work(
                "Updating task",
                window,
                cx,
                move || api.update_task(&task.gid, json!({"completed":!task.completed})),
                |this, task, window, cx| this.apply_clean_update(task, window, cx),
            );
        }
    }

    fn create_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let name = self.draft.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        let project = match &self.page {
            Page::Project(gid) => Some(gid.clone()),
            _ => None,
        };
        if self.demo {
            let me = self.user.as_ref().map(|u| Named {
                gid: u.gid.clone(),
                name: u.name.clone(),
            });
            let projects = self
                .data
                .projects
                .iter()
                .filter(|p| Some(&p.gid) == project.as_ref())
                .map(|p| Named {
                    gid: p.gid.clone(),
                    name: p.name.clone(),
                })
                .collect();
            self.upsert(Task {
                gid: format!(
                    "demo-new-{}",
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                name,
                assignee: me,
                projects,
                modified_at: Some(Utc::now()),
                ..Default::default()
            });
            self.draft.update(cx, |s, cx| s.set_value("", window, cx));
            cx.notify();
        } else if let Some(api) = self.api.clone() {
            let workspace = self.data.workspace.gid.clone();
            self.work(
                "Creating task",
                window,
                cx,
                move || api.create_task(&workspace, project.as_deref(), &name),
                |this, task, window, cx| {
                    this.upsert(task);
                    this.draft.update(cx, |s, cx| s.set_value("", window, cx));
                },
            );
        }
    }

    fn save_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let Some(mut task) = self.selected.clone() else {
            return;
        };
        let name = self.title.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.error = Some("A task needs a name.".into());
            cx.notify();
            return;
        }
        let due = self.due.read(cx).value().trim().to_string();
        let date = if due.is_empty() {
            None
        } else {
            match NaiveDate::parse_from_str(&due, "%Y-%m-%d") {
                Ok(date) if date.to_string() == due => Some(date),
                _ => {
                    self.error = Some("Enter the due date as YYYY-MM-DD, or clear it.".into());
                    cx.notify();
                    return;
                }
            }
        };
        let notes = self.description.read(cx).value().to_string();
        let patch = task.edit_patch(&name, &notes, date, &self.assignee);
        if patch.as_object().is_none_or(|fields| fields.is_empty()) {
            return;
        }
        if self.demo {
            task.name = name;
            task.notes = notes;
            task.assignee = self.assignee.clone();
            task.due_on = date;
            task.due_at = None;
            task.modified_at = Some(Utc::now());
            self.upsert(task.clone());
            self.title
                .update(cx, |s, cx| s.set_value(task.name, window, cx));
            self.error = None;
            cx.notify();
        } else if let Some(api) = self.api.clone() {
            self.work(
                "Saving task",
                window,
                cx,
                move || api.update_task(&task.gid, patch),
                |this, task, window, cx| {
                    this.upsert(task.clone());
                    let comment = this.comment.read(cx).value().to_string();
                    this.set_editor(task, window, cx);
                    this.comment
                        .update(cx, |s, cx| s.set_value(comment, window, cx));
                },
            );
        }
    }

    fn post_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let Some(task) = self.selected.clone() else {
            return;
        };
        let text = self.comment.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        if self.demo {
            self.stories.push(Story {
                gid: format!(
                    "demo-comment-{}",
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                text,
                resource_subtype: "comment_added".into(),
                created_at: Some(Utc::now()),
                created_by: self.user.as_ref().map(|u| Named {
                    gid: u.gid.clone(),
                    name: u.name.clone(),
                }),
            });
            self.comment.update(cx, |s, cx| s.set_value("", window, cx));
            cx.notify();
        } else if let Some(api) = self.api.clone() {
            self.work(
                "Posting comment",
                window,
                cx,
                move || api.comment(&task.gid, &text),
                |this, story, window, cx| {
                    this.stories.push(story);
                    this.comment.update(cx, |s, cx| s.set_value("", window, cx));
                },
            );
        }
    }

    fn move_section(&mut self, section: Named, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        let Some(mut task) = self.selected.clone() else {
            return;
        };
        let Some(project) = self.project.as_ref().map(|p| p.project.gid.clone()) else {
            return;
        };
        if self.demo {
            for membership in &mut task.memberships {
                if membership.project.gid == project {
                    membership.section = Some(section.clone());
                }
            }
            self.apply_clean_update(task, window, cx);
            cx.notify();
        } else if let Some(api) = self.api.clone() {
            self.work(
                "Moving task",
                window,
                cx,
                move || api.move_to_section(&task.gid, &section.gid),
                |this, task, window, cx| this.apply_clean_update(task, window, cx),
            );
        }
    }

    fn create_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let name = self.project_draft.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        if self.demo {
            let project = Project {
                gid: format!("demo-project-{}", Utc::now().timestamp_millis()),
                name,
                ..Default::default()
            };
            self.data.projects.push(project.clone());
            self.demo_pages.push(ProjectData {
                project,
                ..Default::default()
            });
            self.project_draft
                .update(cx, |s, cx| s.set_value("", window, cx));
            cx.notify();
        } else if let Some(api) = self.api.clone() {
            let workspace = self.data.workspace.gid.clone();
            self.work(
                "Creating private project",
                window,
                cx,
                move || api.create_project(&workspace, &name),
                |this, project, window, cx| {
                    this.data.projects.push(project);
                    this.project_draft
                        .update(cx, |s, cx| s.set_value("", window, cx));
                },
            );
        }
    }

    fn logout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        self.cancel_signin(cx);
        self.credential_generation += 1;
        let previous = self.credential_write.take();
        self.busy = Some("Signing out".into());
        cx.spawn_in(window, async move |view, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            let result = match cx.update(|_, cx| cx.read_credentials(oauth::CREDENTIAL_SERVICE)) {
                Ok(read) => match read.await {
                    Ok(None) => Ok(()),
                    Ok(Some(_)) => {
                        match cx.update(|_, cx| cx.delete_credentials(oauth::CREDENTIAL_SERVICE)) {
                            Ok(remove) => remove.await,
                            Err(error) => Err(error),
                        }
                    }
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            };
            let _ = view.update_in(cx, |this, window, cx| {
                this.busy = None;
                match result {
                    Ok(()) => {
                        this.api = None;
                        this.user = None;
                        this.data = WorkspaceData::default();
                        this.workspaces.clear();
                        this.project = None;
                        this.selected = None;
                        this.prefs = Preferences::default();
                        this.stories.clear();
                        this.subtasks.clear();
                        this.notepad.update(cx, |s, cx| s.set_value("", window, cx));
                        this.auth_code
                            .update(cx, |s, cx| s.set_value("", window, cx));
                        this.page = Page::Home;
                        this.status = "Signed out of this device".into();
                    }
                    Err(_) => {
                        this.error = Some(
                            "Could not remove the saved session. Try signing out again.".into(),
                        )
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}
