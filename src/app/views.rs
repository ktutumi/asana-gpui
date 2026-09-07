use super::*;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        input::{Input, Textarea},
        menu::{DropdownMenu, PopupMenuItem},
        spinner::Spinner,
    },
    prelude::FluentBuilder,
};

enum ListRow {
    Section(Named, usize),
    Task(usize),
}

impl WorkspaceApp {
    fn nav_button(
        &self,
        id: &'static str,
        label: &str,
        icon: IconName,
        page: Page,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .w_full()
            .justify_start()
            .icon(icon)
            .label(label.to_string())
            .selected(self.page == page)
            .disabled(self.busy.is_some())
            .on_click(
                cx.listener(move |this, _, window, cx| this.navigate(page.clone(), window, cx)),
            )
    }

    fn render_topbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h_12()
            .w_full()
            .px_4()
            .gap_3()
            .flex_shrink_0()
            .bg(cx.theme().sidebar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("sidebar-toggle")
                    .ghost()
                    .icon(IconName::Menu)
                    .accessibility_label("Toggle sidebar")
                    .tooltip("Toggle sidebar")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sidebar_visible = !this.sidebar_visible;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("create")
                    .icon(IconName::Plus)
                    .label("Create…")
                    .disabled(self.user.is_none() || self.busy.is_some())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.navigate(Page::MyTasks, window, cx);
                        this.draft.update(cx, |s, cx| s.focus(window, cx));
                    })),
            )
            .child(div().flex_1())
            .child(
                div().w(rems(34.)).child(
                    Input::new(&self.query)
                        .prefix(IconName::Search)
                        .aria_label("Search loaded tasks and projects")
                        .disabled(self.user.is_none()),
                ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("⌘ K"),
            )
            .child(div().flex_1())
            .when(self.demo, |d| {
                d.child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(cx.theme().muted)
                        .text_xs()
                        .child("Demo workspace"),
                )
            })
            .when_some(self.user.as_ref(), |d, user| {
                d.child(
                    Button::new("account")
                        .ghost()
                        .label(initials(&user.name))
                        .accessibility_label("Account settings")
                        .tooltip("Account settings")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.navigate(Page::Settings, window, cx)
                        })),
                )
            })
    }

    fn render_rail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(rems(3.5))
            .flex_shrink_0()
            .h_full()
            .py_4()
            .items_center()
            .gap_1()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("work-rail")
                    .ghost()
                    .selected(true)
                    .icon(IconName::Asterisk)
                    .accessibility_label("Work")
                    .tooltip("Work")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.navigate(Page::Home, window, cx)),
                    ),
            )
            .child(div().text_xs().child("Work"))
            .child(div().flex_1())
            .child(
                Button::new("theme-toggle")
                    .ghost()
                    .icon(if self.prefs.light_mode {
                        IconName::Moon
                    } else {
                        IconName::Sun
                    })
                    .accessibility_label("Switch appearance")
                    .tooltip("Switch appearance")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.prefs.light_mode = !this.prefs.light_mode;
                        crate::theme::apply(this.prefs.light_mode, cx);
                        this.save_preferences();
                        cx.notify();
                    })),
            )
            .child(
                Button::new("settings-rail")
                    .ghost()
                    .icon(IconName::Settings)
                    .accessibility_label("Settings")
                    .tooltip("Settings")
                    .on_click(
                        cx.listener(|this, _, window, cx| {
                            this.navigate(Page::Settings, window, cx)
                        }),
                    ),
            )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        let workspaces = self.workspaces.clone();
        let current = self.data.workspace.gid.clone();
        v_flex()
            .w_56()
            .h_full()
            .flex_shrink_0()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .px_4()
                    .pt_4()
                    .pb_2()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("Work"),
            )
            .child(
                v_flex()
                    .px_2()
                    .gap_1()
                    .child(self.nav_button(
                        "nav-home",
                        "Home",
                        IconName::LayoutDashboard,
                        Page::Home,
                        cx,
                    ))
                    .child(self.nav_button("nav-inbox", "Inbox", IconName::Inbox, Page::Inbox, cx))
                    .child(div().my_2().border_b_1().border_color(cx.theme().border))
                    .child(self.nav_button(
                        "nav-tasks",
                        "My tasks",
                        IconName::CircleCheck,
                        Page::MyTasks,
                        cx,
                    ))
                    .child(self.nav_button(
                        "nav-projects",
                        "Projects",
                        IconName::FolderClosed,
                        Page::Projects,
                        cx,
                    )),
            )
            .child(
                h_flex()
                    .px_4()
                    .pt_6()
                    .pb_2()
                    .justify_between()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Projects")
                    .child(
                        Button::new("new-project-nav")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .accessibility_label("Create a project…")
                            .tooltip("Create a project…")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.navigate(Page::Projects, window, cx)
                            })),
                    ),
            )
            .child(
                v_flex()
                    .id("sidebar-projects")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .gap_1()
                    .children(self.data.projects.iter().map(|project| {
                        let gid = project.gid.clone();
                        Button::new(SharedString::from(format!("nav-project-{gid}")))
                            .ghost()
                            .w_full()
                            .justify_start()
                            .icon(if self.prefs.starred.contains(&gid) {
                                IconName::Star
                            } else {
                                IconName::Folder
                            })
                            .label(project.name.clone())
                            .tooltip(project.name.clone())
                            .selected(self.page == Page::Project(gid.clone()))
                            .disabled(self.busy.is_some())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.navigate(Page::Project(gid.clone()), window, cx)
                            }))
                    })),
            )
            .child(
                div()
                    .p_3()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("workspace-picker")
                            .ghost()
                            .w_full()
                            .justify_start()
                            .icon(IconName::Building2)
                            .label(self.data.workspace.name.clone())
                            .disabled(self.busy.is_some() || self.demo)
                            .dropdown_menu(move |mut menu, _, _| {
                                for workspace in &workspaces {
                                    let ws = workspace.clone();
                                    let weak = weak.clone();
                                    menu = menu.item(
                                        PopupMenuItem::new(ws.name.clone())
                                            .checked(ws.gid == current)
                                            .on_click(move |_, window, cx| {
                                                let _ = weak.update(cx, |this, cx| {
                                                    this.load_workspace(ws.clone(), window, cx)
                                                });
                                            }),
                                    );
                                }
                                menu
                            }),
                    ),
            )
    }

    fn render_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("connection-status")
            .role(Role::Status)
            .aria_label(self.busy.as_ref().unwrap_or(&self.status).clone())
            .px_5()
            .h_8()
            .gap_2()
            .flex_shrink_0()
            .border_t_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .when_some(self.busy.as_ref(), |d, label| {
                d.child(Spinner::new().small()).child(label.clone())
            })
            .when(self.busy.is_none(), |d| {
                d.child(if self.demo {
                    "Changes stay in this demo".to_string()
                } else {
                    self.status.clone()
                })
            })
            .child(div().flex_1())
            .child("⌘ N  New task     ⌘ R  Refresh     Esc  Close details")
    }

    fn render_error(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("error-message")
            .role(Role::Alert)
            .aria_label(self.error.clone().unwrap_or_default())
            .px_5()
            .py_3()
            .gap_3()
            .bg(cx.theme().muted)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(Icon::new(IconName::TriangleAlert).text_color(cx.theme().warning))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .child(self.error.clone().unwrap_or_default()),
            )
            .child(
                Button::new("dismiss-error")
                    .ghost()
                    .small()
                    .icon(IconName::Close)
                    .accessibility_label("Dismiss message")
                    .tooltip("Dismiss message")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.error = None;
                        cx.notify();
                    })),
            )
    }

    fn render_home(&self, cx: &mut Context<Self>) -> AnyElement {
        let today = Local::now().date_naive();
        let hour = chrono::Timelike::hour(&Local::now());
        let greeting = if hour < 12 {
            "Good morning"
        } else if hour < 18 {
            "Good afternoon"
        } else {
            "Good evening"
        };
        let first = self
            .user
            .as_ref()
            .and_then(|u| u.name.split_whitespace().next())
            .unwrap_or("there");
        let query = self.query.read(cx).value().to_lowercase();
        let tasks: Vec<_> = self
            .data
            .tasks
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&query))
            .filter(|t| match self.home_filter {
                TaskFilter::Incomplete => !t.completed && !t.overdue(today),
                filter => filter.matches(t, today),
            })
            .collect();
        let completed = self.data.tasks.iter().filter(|t| t.completed).count();
        let overdue = self.data.tasks.iter().filter(|t| t.overdue(today)).count();
        let mut sorted = tasks;
        sorted.sort_by_key(|t| (t.due_date().is_none(), t.due_date()));
        v_flex()
            .id("home-scroll")
            .size_full()
            .overflow_y_scroll()
            .child(
                v_flex()
                    .p_8()
                    .gap_6()
                    .max_w(rems(100.))
                    .w_full()
                    .mx_auto()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(today.format("%A, %B %-d").to_string()),
                            )
                            .child(
                                h_flex()
                                    .justify_between()
                                    .gap_4()
                                    .child(div().text_3xl().child(format!("{greeting}, {first}")))
                                    .child(
                                        h_flex()
                                            .gap_5()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("{completed} tasks completed"))
                                            .child(format!("{overdue} overdue")),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_stretch()
                            .gap_5()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .min_h(rems(27.))
                                    .rounded_lg()
                                    .bg(cx.theme().sidebar)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_flex()
                                            .p_5()
                                            .gap_3()
                                            .child(
                                                self.avatar(
                                                    &self
                                                        .user
                                                        .as_ref()
                                                        .map(|u| u.name.clone())
                                                        .unwrap_or_default(),
                                                    cx,
                                                ),
                                            )
                                            .child(
                                                div().text_lg().font_semibold().child("My tasks"),
                                            ),
                                    )
                                    .child(
                                        h_flex().px_4().pb_3().gap_1().children(
                                            [
                                                (TaskFilter::Incomplete, "Upcoming"),
                                                (TaskFilter::Overdue, "Overdue"),
                                                (TaskFilter::Completed, "Completed"),
                                            ]
                                            .into_iter()
                                            .map(
                                                |(filter, label)| {
                                                    Button::new(SharedString::from(format!(
                                                        "home-{label}"
                                                    )))
                                                    .ghost()
                                                    .small()
                                                    .label(label)
                                                    .selected(self.home_filter == filter)
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.home_filter = filter;
                                                        cx.notify();
                                                    }))
                                                },
                                            ),
                                        ),
                                    )
                                    .child(
                                        div().px_5().pb_3().child(
                                            Button::new("home-add")
                                                .ghost()
                                                .small()
                                                .icon(IconName::Plus)
                                                .label("Create task…")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.navigate(Page::MyTasks, window, cx);
                                                    this.draft
                                                        .update(cx, |s, cx| s.focus(window, cx));
                                                })),
                                        ),
                                    )
                                    .children(
                                        sorted.iter().take(7).map(|t| self.compact_task(t, cx)),
                                    )
                                    .when(sorted.is_empty(), |d| {
                                        d.child(self.empty(
                                            "Nothing here yet",
                                            "Your tasks will appear here as you plan your work.",
                                            IconName::CircleCheck,
                                            cx,
                                        ))
                                    })
                                    .child(div().flex_1())
                                    .child(
                                        div().px_5().py_3().child(
                                            Button::new("all-my-tasks")
                                                .ghost()
                                                .small()
                                                .label("View all my tasks")
                                                .icon(IconName::ArrowRight)
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.navigate(Page::MyTasks, window, cx)
                                                })),
                                        ),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .rounded_lg()
                                    .bg(cx.theme().sidebar)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_flex()
                                            .p_5()
                                            .gap_3()
                                            .child(
                                                div().text_lg().font_semibold().child("Projects"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Your workspace"),
                                            ),
                                    )
                                    .children(
                                        self.data
                                            .projects
                                            .iter()
                                            .filter(|p| p.name.to_lowercase().contains(&query))
                                            .take(6)
                                            .map(|p| self.project_card(p, false, cx)),
                                    )
                                    .when(self.data.projects.is_empty(), |d| {
                                        d.child(self.empty(
                                            "Make room for your next project",
                                            "Organize tasks around a shared outcome.",
                                            IconName::Folder,
                                            cx,
                                        ))
                                    })
                                    .child(div().flex_1())
                                    .child(
                                        div().p_4().child(
                                            Button::new("home-projects")
                                                .ghost()
                                                .small()
                                                .label("Browse projects")
                                                .icon(IconName::ArrowRight)
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.navigate(Page::Projects, window, cx)
                                                })),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_stretch()
                            .gap_5()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .p_5()
                                    .gap_4()
                                    .rounded_lg()
                                    .bg(cx.theme().sidebar)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(div().text_lg().font_semibold().child("People"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("The people in your workspace"),
                                    )
                                    .children(self.data.users.iter().take(5).map(|u| {
                                        h_flex()
                                            .gap_3()
                                            .child(self.avatar(&u.name, cx))
                                            .child(u.name.clone())
                                    })),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .p_5()
                                    .gap_3()
                                    .rounded_lg()
                                    .bg(cx.theme().sidebar)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .font_semibold()
                                                    .child("Private notepad"),
                                            )
                                            .child(
                                                Button::new("save-notepad")
                                                    .ghost()
                                                    .small()
                                                    .label("Save note")
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        let previous = this.prefs.notes.clone();
                                                        this.prefs.notes = this
                                                            .notepad
                                                            .read(cx)
                                                            .value()
                                                            .to_string();
                                                        if this.save_preferences() {
                                                            this.status =
                                                                "Note saved on this device".into();
                                                        } else {
                                                            this.prefs.notes = previous;
                                                        }
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                    .child(
                                        Textarea::new(&self.notepad)
                                            .h(rems(10.))
                                            .appearance(false)
                                            .aria_label("Private notepad"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Only on this device"),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn avatar(&self, name: &str, cx: &App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .size_8()
            .flex_shrink_0()
            .rounded_full()
            .bg(cx.theme().primary)
            .text_color(cx.theme().primary_foreground)
            .text_xs()
            .child(initials(name))
    }

    fn empty(&self, title: &str, description: &str, icon: IconName, cx: &App) -> impl IntoElement {
        v_flex()
            .p_8()
            .gap_3()
            .items_center()
            .justify_center()
            .text_center()
            .text_color(cx.theme().muted_foreground)
            .child(Icon::new(icon).large())
            .child(
                div()
                    .text_base()
                    .text_color(cx.theme().foreground)
                    .child(title.to_string()),
            )
            .child(div().text_sm().child(description.to_string()))
    }

    pub(super) fn compact_task(
        &self,
        task: &Task,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let toggle = task.clone();
        let open = task.clone();
        h_flex()
            .h_12()
            .px_4()
            .gap_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                Button::new(SharedString::from(format!("home-complete-{}", task.gid)))
                    .ghost()
                    .xsmall()
                    .icon(IconName::CircleCheck)
                    .selected(task.completed)
                    .accessibility_label(if task.completed {
                        "Mark incomplete"
                    } else {
                        "Mark complete"
                    })
                    .tooltip(if task.completed {
                        "Mark incomplete"
                    } else {
                        "Mark complete"
                    })
                    .disabled(self.busy.is_some())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.toggle_task(toggle.clone(), window, cx)
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("home-open-{}", task.gid)))
                    .ghost()
                    .small()
                    .flex_1()
                    .min_w_0()
                    .justify_start()
                    .accessibility_label(task.name.clone())
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .text_left()
                            .child(task.name.clone()),
                    )
                    .tooltip(task.name.clone())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_task(open.clone(), window, cx)
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(if task.overdue(Local::now().date_naive()) {
                        cx.theme().danger
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(date_label(task.due_date(), Local::now().date_naive())),
            )
    }

    fn project_card(
        &self,
        project: &Project,
        large: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let gid = project.gid.clone();
        Button::new(SharedString::from(format!("project-card-{gid}")))
            .accessibility_label(project.name.clone())
            .ghost()
            .h_auto()
            .py_4()
            .px_5()
            .w_full()
            .justify_start()
            .disabled(self.busy.is_some())
            .tooltip(project.name.clone())
            .child(
                h_flex()
                    .w_full()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size_10()
                            .rounded_lg()
                            .bg(cx.theme().muted)
                            .child(Icon::new(IconName::Folder).text_color(cx.theme().primary)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .items_start()
                            .gap_1()
                            .child(div().font_semibold().truncate().child(project.name.clone()))
                            .when(large, |d| {
                                d.child(
                                    div()
                                        .text_sm()
                                        .font_normal()
                                        .text_color(cx.theme().muted_foreground)
                                        .truncate()
                                        .child(project.notes.clone()),
                                )
                            }),
                    ),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.navigate(Page::Project(gid.clone()), window, cx)
            }))
    }

    fn source_tasks(&self) -> &[Task] {
        if matches!(self.page, Page::Project(_)) {
            self.project
                .as_ref()
                .map(|p| p.tasks.as_slice())
                .unwrap_or_default()
        } else {
            &self.data.tasks
        }
    }

    fn groups(&self, cx: &App) -> Vec<(Named, Vec<usize>)> {
        let query = self.query.read(cx).value().to_lowercase();
        let tasks = self.source_tasks();
        let today = Local::now().date_naive();
        let project = match &self.page {
            Page::Project(gid) => Some(gid.as_str()),
            _ => None,
        };
        let mut groups: Vec<(Named, Vec<usize>)> = if project.is_some() {
            self.project
                .as_ref()
                .map(|p| p.sections.iter().map(|s| (s.clone(), Vec::new())).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        for (ix, task) in tasks.iter().enumerate() {
            if !self.filter.matches(task, today) || !task.name.to_lowercase().contains(&query) {
                continue;
            }
            let name = task.section(project);
            if let Some((_, items)) = groups.iter_mut().find(|(group, _)| group.gid == name.gid) {
                items.push(ix);
            } else {
                groups.push((name, vec![ix]));
            }
        }
        if self.sort_by_due {
            for (_, items) in &mut groups {
                items.sort_by_key(|ix| (tasks[*ix].due_date().is_none(), tasks[*ix].due_date()));
            }
        }
        groups
    }

    fn render_task_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let project = matches!(self.page, Page::Project(_));
        let name = if project {
            self.project
                .as_ref()
                .map(|p| p.project.name.clone())
                .unwrap_or_else(|| "Project".into())
        } else {
            "My tasks".into()
        };
        let tabs = if project {
            vec![
                (View::Overview, "Overview", IconName::Info),
                (View::List, "List", IconName::Menu),
                (View::Board, "Board", IconName::GalleryVerticalEnd),
                (View::Calendar, "Calendar", IconName::Calendar),
                (View::Dashboard, "Dashboard", IconName::ChartPie),
            ]
        } else {
            vec![
                (View::List, "List", IconName::Menu),
                (View::Board, "Board", IconName::GalleryVerticalEnd),
                (View::Calendar, "Calendar", IconName::Calendar),
            ]
        };
        let mut header = h_flex()
            .px_6()
            .pt_5()
            .pb_3()
            .gap_3()
            .child(
                Icon::new(if project {
                    IconName::Folder
                } else {
                    IconName::CircleCheck
                })
                .large()
                .text_color(cx.theme().primary),
            )
            .child(div().text_2xl().font_semibold().child(name));
        if let Page::Project(gid) = &self.page {
            let gid = gid.clone();
            let starred = self.prefs.starred.contains(&gid);
            header = header.child(
                Button::new("star-project")
                    .ghost()
                    .small()
                    .icon(if starred {
                        IconName::StarFill
                    } else {
                        IconName::Star
                    })
                    .tooltip(if starred {
                        "Remove from starred"
                    } else {
                        "Add to starred"
                    })
                    .accessibility_label(if starred {
                        "Remove from starred"
                    } else {
                        "Add to starred"
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.prefs.starred.insert(gid.clone()) {
                            this.prefs.starred.remove(&gid);
                        }
                        this.save_preferences();
                        cx.notify();
                    })),
            );
        }
        v_flex()
            .size_full()
            .child(header)
            .child(
                h_flex()
                    .px_5()
                    .gap_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .children(tabs.into_iter().map(|(view, label, icon)| {
                        Button::new(SharedString::from(format!("view-{label}")))
                            .ghost()
                            .label(label)
                            .icon(icon)
                            .selected(self.view == view)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.view = view;
                                cx.notify();
                            }))
                    })),
            )
            .when(
                !matches!(self.view, View::Overview | View::Dashboard),
                |d| d.child(self.render_task_toolbar(cx)),
            )
            .child(match self.view {
                View::Overview => self.render_overview(cx),
                View::List => self.render_list(cx),
                View::Board => self.render_board(cx),
                View::Calendar => self.render_calendar(cx),
                View::Dashboard => self.render_dashboard(cx),
            })
            .into_any_element()
    }

    fn render_task_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        let current = self.filter;
        h_flex()
            .px_6()
            .py_3()
            .gap_2()
            .flex_shrink_0()
            .child(
                div().max_w(rems(30.)).flex_1().child(
                    Input::new(&self.draft)
                        .aria_label("New task name")
                        .disabled(self.busy.is_some()),
                ),
            )
            .child(
                Button::new("add-task")
                    .icon(IconName::Plus)
                    .label("Add task")
                    .disabled(self.busy.is_some() || self.draft.read(cx).value().trim().is_empty())
                    .on_click(cx.listener(|this, _, window, cx| this.create_task(window, cx))),
            )
            .child(div().flex_1())
            .child(
                Button::new("filter")
                    .ghost()
                    .small()
                    .label(self.filter.label())
                    .icon(IconName::ChevronDown)
                    .dropdown_menu(move |mut menu, _, _| {
                        for filter in [
                            TaskFilter::Incomplete,
                            TaskFilter::All,
                            TaskFilter::Completed,
                            TaskFilter::Overdue,
                        ] {
                            let weak = weak.clone();
                            menu = menu.item(
                                PopupMenuItem::new(filter.label())
                                    .checked(current == filter)
                                    .on_click(move |_, _, cx| {
                                        let _ = weak.update(cx, |this, cx| {
                                            this.filter = filter;
                                            cx.notify();
                                        });
                                    }),
                            );
                        }
                        menu
                    }),
            )
            .child(
                Button::new("sort-due")
                    .ghost()
                    .small()
                    .label("Due date")
                    .icon(IconName::SortAscending)
                    .selected(self.sort_by_due)
                    .accessibility_label("Sort by due date")
                    .tooltip("Sort by due date")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sort_by_due = !this.sort_by_due;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("refresh")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .accessibility_label("Refresh · ⌘ R")
                    .tooltip("Refresh · ⌘ R")
                    .disabled(self.busy.is_some())
                    .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
            )
    }

    fn render_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let groups = self.groups(cx);
        let mut rows = Vec::new();
        for (name, items) in groups {
            rows.push(ListRow::Section(name.clone(), items.len()));
            if !self.collapsed.contains(&name.gid) {
                rows.extend(items.into_iter().map(ListRow::Task));
            }
        }
        if rows.is_empty() {
            return self
                .empty(
                    "No matching tasks",
                    "Change the filter or create your first task.",
                    IconName::CircleCheck,
                    cx,
                )
                .into_any_element();
        }
        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                h_flex()
                    .h_9()
                    .px_6()
                    .gap_3()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .border_y_1()
                    .border_color(cx.theme().border)
                    .child(div().w_6())
                    .child(div().flex_1().child("Name"))
                    .child(div().w_40().child(if matches!(self.page, Page::MyTasks) {
                        "Projects"
                    } else {
                        "Assignee"
                    }))
                    .child(div().w_24().child("Due date")),
            )
            .child(
                uniform_list(
                    "tasks-list",
                    rows.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|ix| match &rows[ix] {
                                ListRow::Section(name, count) => {
                                    let gid = name.gid.clone();
                                    h_flex()
                                        .h_11()
                                        .w_full()
                                        .px_5()
                                        .gap_2()
                                        .child(
                                            Button::new(SharedString::from(format!(
                                                "section-{}",
                                                name.gid
                                            )))
                                            .ghost()
                                            .small()
                                            .icon(if this.collapsed.contains(&name.gid) {
                                                IconName::ChevronRight
                                            } else {
                                                IconName::ChevronDown
                                            })
                                            .label(name.name.clone())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if !this.collapsed.insert(gid.clone()) {
                                                    this.collapsed.remove(&gid);
                                                }
                                                cx.notify();
                                            })),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(count.to_string()),
                                        )
                                        .into_any_element()
                                }
                                ListRow::Task(ix) => {
                                    let task = this.source_tasks()[*ix].clone();
                                    this.task_row(&task, cx).into_any_element()
                                }
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1()
                .w_full(),
            )
            .into_any_element()
    }

    fn task_row(&self, task: &Task, cx: &mut Context<Self>) -> impl IntoElement {
        let toggle = task.clone();
        let open = task.clone();
        let selected = self.selected.as_ref().is_some_and(|t| t.gid == task.gid);
        h_flex()
            .h_11()
            .w_full()
            .px_6()
            .gap_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(selected, |d| d.bg(cx.theme().muted))
            .child(
                Button::new(SharedString::from(format!("complete-{}", task.gid)))
                    .ghost()
                    .xsmall()
                    .icon(IconName::CircleCheck)
                    .selected(task.completed)
                    .accessibility_label(if task.completed {
                        "Mark incomplete"
                    } else {
                        "Mark complete"
                    })
                    .tooltip(if task.completed {
                        "Mark incomplete"
                    } else {
                        "Mark complete"
                    })
                    .disabled(self.busy.is_some())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.toggle_task(toggle.clone(), window, cx)
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("task-{}", task.gid)))
                    .ghost()
                    .small()
                    .flex_1()
                    .min_w_0()
                    .justify_start()
                    .accessibility_label(task.name.clone())
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .text_left()
                            .child(task.name.clone()),
                    )
                    .tooltip(task.name.clone())
                    .when(task.completed, |b| {
                        b.text_color(cx.theme().muted_foreground)
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_task(open.clone(), window, cx)
                    })),
            )
            .child(
                div()
                    .w_40()
                    .flex_shrink_0()
                    .text_xs()
                    .truncate()
                    .text_color(cx.theme().muted_foreground)
                    .child(if matches!(self.page, Page::MyTasks) {
                        task.projects
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    } else {
                        task.assignee
                            .as_ref()
                            .map(|u| u.name.clone())
                            .unwrap_or_else(|| "Unassigned".into())
                    }),
            )
            .child(
                div()
                    .w_24()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(if task.overdue(Local::now().date_naive()) {
                        cx.theme().danger
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(date_label(task.due_date(), Local::now().date_naive())),
            )
    }

    fn render_board(&self, cx: &mut Context<Self>) -> AnyElement {
        let groups = self.groups(cx);
        h_flex()
            .id("board-scroll")
            .items_stretch()
            .flex_1()
            .min_h_0()
            .overflow_x_scroll()
            .p_5()
            .gap_5()
            .children(groups.into_iter().map(|(name, items)| {
                v_flex()
                    .w_72()
                    .h_full()
                    .flex_shrink_0()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_3()
                            .px_2()
                            .font_semibold()
                            .child(name.name.clone())
                            .child(
                                div()
                                    .text_xs()
                                    .font_normal()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(items.len().to_string()),
                            ),
                    )
                    .child(
                        v_flex()
                            .id(SharedString::from(format!("board-{}", name.gid)))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .gap_3()
                            .children(items.iter().map(|ix| {
                                let task = &self.source_tasks()[*ix];
                                let open = task.clone();
                                Button::new(SharedString::from(format!("card-{}", task.gid)))
                                    .accessibility_label(task.name.clone())
                                    .h_auto()
                                    .p_4()
                                    .w_full()
                                    .justify_start()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_task(open.clone(), window, cx)
                                    }))
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .items_start()
                                            .gap_4()
                                            .child(
                                                h_flex()
                                                    .w_full()
                                                    .gap_2()
                                                    .items_start()
                                                    .child(
                                                        Icon::new(IconName::CircleCheck)
                                                            .small()
                                                            .text_color(if task.completed {
                                                                cx.theme().success
                                                            } else {
                                                                cx.theme().muted_foreground
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .whitespace_normal()
                                                            .text_left()
                                                            .child(task.name.clone()),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .w_full()
                                                    .justify_between()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(date_label(
                                                        task.due_date(),
                                                        Local::now().date_naive(),
                                                    ))
                                                    .child(
                                                        task.assignee
                                                            .as_ref()
                                                            .map(|u| initials(&u.name))
                                                            .unwrap_or_default(),
                                                    ),
                                            ),
                                    )
                            })),
                    )
            }))
            .into_any_element()
    }

    fn render_calendar(&self, cx: &mut Context<Self>) -> AnyElement {
        let first = self.calendar_month;
        let start = first - chrono::Duration::days(first.weekday().num_days_from_monday().into());
        let tasks = self.source_tasks();
        let query = self.query.read(cx).value().to_lowercase();
        let today = Local::now().date_naive();
        v_flex()
            .flex_1()
            .min_h_0()
            .child(
                h_flex()
                    .px_6()
                    .pb_4()
                    .gap_3()
                    .child(
                        Button::new("previous-month")
                            .ghost()
                            .small()
                            .icon(IconName::ChevronLeft)
                            .accessibility_label("Previous month")
                            .tooltip("Previous month")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.calendar_month = (this.calendar_month
                                    - chrono::Duration::days(1))
                                .with_day(1)
                                .unwrap();
                                cx.notify();
                            })),
                    )
                    .child(div().text_lg().child(first.format("%B %Y").to_string()))
                    .child(
                        Button::new("next-month")
                            .ghost()
                            .small()
                            .icon(IconName::ChevronRight)
                            .accessibility_label("Next month")
                            .tooltip("Next month")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.calendar_month = (this.calendar_month
                                    + chrono::Duration::days(32))
                                .with_day(1)
                                .unwrap();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("this-month")
                            .ghost()
                            .small()
                            .label("Today")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.calendar_month =
                                    Local::now().date_naive().with_day(1).unwrap();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex().px_5().children(
                    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                        .into_iter()
                        .map(|day| {
                            div()
                                .flex_1()
                                .p_2()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(day)
                        }),
                ),
            )
            .child(
                v_flex()
                    .id("calendar-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_5()
                    .pb_5()
                    .children((0..6).map(|week| {
                        h_flex().items_stretch().children((0..7).map(|day| {
                            let date = start + chrono::Duration::days(week * 7 + day);
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .min_h(rems(7.))
                                .p_2()
                                .gap_1()
                                .border_1()
                                .border_color(cx.theme().border)
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(if date == today {
                                            cx.theme().primary
                                        } else if date.month() != first.month() {
                                            cx.theme().muted_foreground
                                        } else {
                                            cx.theme().foreground
                                        })
                                        .child(date.day().to_string()),
                                )
                                .children(
                                    tasks
                                        .iter()
                                        .filter(|t| {
                                            t.due_date() == Some(date)
                                                && self.filter.matches(t, today)
                                                && t.name.to_lowercase().contains(&query)
                                        })
                                        .map(|task| {
                                            let open = task.clone();
                                            Button::new(SharedString::from(format!(
                                                "calendar-{}",
                                                task.gid
                                            )))
                                            .ghost()
                                            .xsmall()
                                            .w_full()
                                            .justify_start()
                                            .label(task.name.clone())
                                            .tooltip(task.name.clone())
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.open_task(open.clone(), window, cx)
                                            }))
                                        }),
                                )
                        }))
                    })),
            )
            .into_any_element()
    }

    fn render_overview(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(project) = &self.project else {
            return div().into_any_element();
        };
        v_flex()
            .id("overview")
            .flex_1()
            .overflow_y_scroll()
            .p_8()
            .gap_6()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("PROJECT OVERVIEW"),
            )
            .child(div().text_2xl().child(project.project.name.clone()))
            .child(
                h_flex()
                    .gap_8()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Project owner"),
                            )
                            .child(
                                project
                                    .project
                                    .owner
                                    .as_ref()
                                    .map(|u| u.name.clone())
                                    .unwrap_or_else(|| "Unassigned".into()),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Due date"),
                            )
                            .child(date_label(
                                project.project.due_on,
                                Local::now().date_naive(),
                            )),
                    ),
            )
            .child(div().h_1().border_b_1().border_color(cx.theme().border))
            .child(div().text_lg().font_semibold().child("Project description"))
            .child(div().max_w(rems(60.)).text_sm().whitespace_normal().child(
                if project.project.notes.is_empty() {
                    "No description yet.".to_string()
                } else {
                    project.project.notes.clone()
                },
            ))
            .when_some(project.project.current_status.as_ref(), |d, status| {
                d.child(
                    v_flex()
                        .p_5()
                        .gap_3()
                        .rounded_lg()
                        .bg(cx.theme().sidebar)
                        .child(div().font_semibold().child(status.title.clone()))
                        .child(status.text.clone()),
                )
            })
            .child(self.render_dashboard(cx))
            .into_any_element()
    }

    fn render_dashboard(&self, cx: &mut Context<Self>) -> AnyElement {
        let tasks = self.source_tasks();
        let done = tasks.iter().filter(|t| t.completed).count();
        let overdue = tasks
            .iter()
            .filter(|t| t.overdue(Local::now().date_naive()))
            .count();
        v_flex()
            .p_6()
            .gap_6()
            .child(
                h_flex().gap_4().items_stretch().children(
                    [
                        ("Total tasks", tasks.len()),
                        ("Completed", done),
                        ("Incomplete", tasks.len() - done),
                        ("Overdue", overdue),
                    ]
                    .into_iter()
                    .map(|(label, count)| {
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .p_5()
                            .gap_3()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded_lg()
                            .bg(cx.theme().sidebar)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(label),
                            )
                            .child(div().text_3xl().child(count.to_string()))
                    }),
                ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(div().font_semibold().child("Task completion"))
                    .child(
                        component::progress::Progress::new("completion-progress").value(
                            if tasks.is_empty() {
                                0.
                            } else {
                                done as f32 / tasks.len() as f32 * 100.
                            },
                        ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{done} of {} tasks completed", tasks.len())),
                    ),
            )
            .into_any_element()
    }

    fn render_projects(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.query.read(cx).value().to_lowercase();
        v_flex()
            .size_full()
            .p_6()
            .gap_5()
            .child(div().text_2xl().font_semibold().child("Projects"))
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        div()
                            .w_80()
                            .child(Input::new(&self.project_draft).aria_label("New project name")),
                    )
                    .child(
                        Button::new("create-project")
                            .label("Create private project")
                            .icon(IconName::Plus)
                            .disabled(self.busy.is_some())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.create_project(window, cx)),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .id("project-directory")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap_2()
                    .children(
                        self.data
                            .projects
                            .iter()
                            .filter(|p| p.name.to_lowercase().contains(&query))
                            .map(|p| {
                                div()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded_lg()
                                    .child(self.project_card(p, true, cx))
                            }),
                    ),
            )
            .into_any_element()
    }

    fn render_inbox(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.query.read(cx).value().to_lowercase();
        let mut tasks: Vec<_> = self
            .data
            .tasks
            .iter()
            .filter(|t| {
                self.prefs.archived.contains(&t.activity_key()) == self.inbox_archive
                    && (!self.inbox_unread || !self.prefs.read.contains(&t.activity_key()))
                    && t.name.to_lowercase().contains(&query)
            })
            .collect();
        tasks.sort_by_key(|t| std::cmp::Reverse(t.modified_at));
        v_flex().size_full().child(div().px_6().pt_5().pb_3().text_2xl().font_semibold().child("Inbox"))
            .child(h_flex().px_5().gap_3().border_b_1().border_color(cx.theme().border)
                .children([(false,"Activity"),(true,"Archive")].into_iter().map(|(archive,label)|Button::new(SharedString::from(format!("inbox-{label}"))).ghost().label(label).selected(self.inbox_archive==archive).on_click(cx.listener(move|this,_,_,cx|{this.inbox_archive=archive;cx.notify();})))))
            .child(h_flex().px_6().py_3().gap_3().child(div().flex_1().text_xs().text_color(cx.theme().muted_foreground).child("Updates to your assigned tasks · read and archive stay on this device"))
                .child(Button::new("unread-filter").ghost().small().label("Unread").selected(self.inbox_unread).on_click(cx.listener(|this,_,_,cx|{this.inbox_unread= !this.inbox_unread;cx.notify();})))
                .child(Button::new("read-all").ghost().small().label("Mark all read").on_click(cx.listener(|this,_,_,cx|{for task in &this.data.tasks{this.prefs.read.insert(task.activity_key());}this.save_preferences();cx.notify();})))
                .child(Button::new("inbox-refresh").ghost().small().icon(IconName::RotateCw).accessibility_label("Refresh activity").tooltip("Refresh activity").disabled(self.busy.is_some()).on_click(cx.listener(|this,_,window,cx|this.refresh(window,cx)))))
            .when(tasks.is_empty(),|d|d.child(self.empty("You're all caught up","Task updates will appear here. Refresh to check for changes.",IconName::Inbox,cx)))
            .child(v_flex().id("inbox-list").flex_1().min_h_0().overflow_y_scroll().children(tasks.into_iter().map(|task|{
                let open=task.clone();let key=task.activity_key();let unread= !self.prefs.read.contains(&key);
                h_flex().items_start().px_6().py_4().gap_4().border_b_1().border_color(cx.theme().border).when(unread,|d|d.bg(cx.theme().sidebar))
                    .child(div().pt_2().child(Icon::new(IconName::Bell).text_color(if unread{cx.theme().primary}else{cx.theme().muted_foreground})))
                    .child(Button::new(SharedString::from(format!("inbox-task-{}",task.gid))).accessibility_label(task.name.clone()).ghost().flex_1().min_w_0().h_auto().justify_start().on_click(cx.listener(move|this,_,window,cx|this.open_task(open.clone(),window,cx)))
                        .child(v_flex().w_full().items_start().gap_2().min_w_0().child(div().text_xs().text_color(cx.theme().muted_foreground).child(task.projects.first().map(|p|p.name.clone()).unwrap_or_else(||"My tasks".into())))
                            .child(div().font_semibold().truncate().child(task.name.clone())).child(div().text_sm().text_color(cx.theme().muted_foreground).child(if task.completed{"Task completed"}else{"Task updated"}))
                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(task.modified_at.map(|d|d.with_timezone(&Local).format("%b %-d · %H:%M").to_string()).unwrap_or_default()))))
                    .child(Button::new(SharedString::from(format!("archive-{}",task.gid))).ghost().small().label(if self.inbox_archive{"Restore"}else{"Archive"}).on_click(cx.listener(move|this,_,_,cx|{if !this.prefs.archived.insert(key.clone()){this.prefs.archived.remove(&key);}this.save_preferences();cx.notify();})))
            }))).into_any_element()
    }

    fn render_connect(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex().id("connect-scroll").size_full().overflow_y_scroll().items_center().justify_center().p_8().gap_6()
            .child(v_flex().max_w(rems(39.)).w_full().gap_5().p_8().rounded_xl().bg(cx.theme().sidebar).border_1().border_color(cx.theme().border)
                .child(Icon::new(IconName::Asterisk).large().text_color(cx.theme().primary))
                .child(div().text_3xl().child("Your work, in focus."))
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Connect Asana to bring your tasks, projects, and conversations into a native workspace."))
                .child(self.connection_fields(cx))
                .child(Button::new("connect-oauth").primary().large().label("Connect with Asana").disabled(self.busy.is_some()||self.pending_auth.is_some()).on_click(cx.listener(|this,_,window,cx|this.connect_oauth(window,cx))))
                .when(self.cancel_auth.is_some(),|d|d.child(Button::new("cancel-oauth").ghost().label("Cancel sign-in").on_click(cx.listener(|this,_,_,cx|this.cancel_signin(cx)))))
                .when(self.pending_auth.is_some(),|d|d.child(Input::new(&self.auth_code).aria_label("Authorization code")).child(Button::new("finish-oauth").label("Finish sign-in").disabled(self.busy.is_some()).on_click(cx.listener(|this,_,window,cx|this.finish_manual(window,cx)))))
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Your browser handles authorization. OAuth tokens are saved in the system credential store."))
                .child(h_flex().gap_3().child(Button::new("try-demo").ghost().label("Explore a demo").disabled(self.busy.is_some()||self.pending_auth.is_some()).on_click(cx.listener(|this,_,window,cx|this.enter_demo(window,cx))))
                    .when(std::env::var_os("ASANA_API_KEY").is_some(),|d|d.child(Button::new("use-environment-token").ghost().label("Use ASANA_API_KEY").disabled(self.busy.is_some()).on_click(cx.listener(|this,_,window,cx|this.connect_pat(window,cx)))))))
            .into_any_element()
    }

    fn connection_fields(&self, cx: &App) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                self.field(
                    "Client ID",
                    Input::new(&self.client_id)
                        .aria_label("Client ID")
                        .disabled(self.busy.is_some()),
                    cx,
                ),
            )
            .child(
                self.field(
                    "Client secret",
                    Input::new(&self.client_secret)
                        .aria_label("Client secret")
                        .disabled(self.busy.is_some()),
                    cx,
                ),
            )
            .child(
                self.field(
                    "Redirect URL",
                    Input::new(&self.redirect)
                        .aria_label("Redirect URL")
                        .disabled(self.busy.is_some()),
                    cx,
                ),
            )
            .child(
                self.field(
                    "Permission scopes",
                    Input::new(&self.scopes)
                        .aria_label("Permission scopes")
                        .disabled(self.busy.is_some()),
                    cx,
                ),
            )
    }

    pub(super) fn field(&self, label: &str, input: impl IntoElement, cx: &App) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(label.to_string()),
            )
            .child(input)
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex().id("settings-scroll").size_full().overflow_y_scroll().p_8().gap_6()
            .child(div().text_2xl().font_semibold().child("Account & appearance"))
            .child(div().text_sm().child(self.user.as_ref().map(|u|format!("Connected as {} · {}",u.name,self.data.workspace.name)).unwrap_or_else(||"Not connected".into())))
            .child(h_flex().gap_3().child(Button::new("appearance-setting").label(if self.prefs.light_mode{"Use dark appearance"}else{"Use light appearance"}).on_click(cx.listener(|this,_,_,cx|{this.prefs.light_mode= !this.prefs.light_mode;crate::theme::apply(this.prefs.light_mode,cx);this.save_preferences();cx.notify();})))
                .child(Button::new("sign-out").label(if self.demo{"Leave demo"}else{"Sign out of this device"}).disabled(self.busy.is_some()).on_click(cx.listener(|this,_,window,cx|{if this.demo{this.user=None;this.demo=false;this.page=Page::Home;cx.notify();}else{this.logout(window,cx);}}))))
            .child(div().max_w(rems(42.)).child(self.connection_fields(cx)))
            .child(h_flex().gap_3().child(Button::new("reconnect").label("Connect with OAuth").disabled(self.busy.is_some()).on_click(cx.listener(|this,_,window,cx|this.connect_oauth(window,cx))))
                .when(self.cancel_auth.is_some(),|d|d.child(Button::new("cancel-reconnect").ghost().label("Cancel sign-in").on_click(cx.listener(|this,_,_,cx|this.cancel_signin(cx))))))
            .when(self.pending_auth.is_some(),|d|d.child(Input::new(&self.auth_code).aria_label("Authorization code")).child(Button::new("finish-reconnect").label("Finish sign-in").on_click(cx.listener(|this,_,window,cx|this.finish_manual(window,cx)))))
            .child(div().max_w(rems(48.)).text_sm().text_color(cx.theme().muted_foreground).child("Inbox shows activity from your assigned tasks. Its read and archive state, starred projects, and private notepad are stored on this device. They do not change the Asana web app's Inbox or Home settings."))
            .into_any_element()
    }
}

impl Render for WorkspaceApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&self.focus)
            .key_context("AsanaWorkspace")
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.quit(cx);
            }))
            .on_action(cx.listener(|this, _: &Refresh, window, cx| this.refresh(window, cx)))
            .on_action(cx.listener(|this, _: &SearchTasks, window, cx| {
                this.query.update(cx, |s, cx| s.focus(window, cx))
            }))
            .on_action(cx.listener(|this, _: &NewTask, window, cx| {
                this.navigate(Page::MyTasks, window, cx);
                this.draft.update(cx, |s, cx| s.focus(window, cx));
            }))
            .on_action(cx.listener(|this, _: &SaveTask, window, cx| this.save_task(window, cx)))
            .on_action(cx.listener(|this, _: &CloseDetail, window, cx| {
                if this.can_leave(cx) {
                    this.selected = None;
                    this.focus.focus(window, cx);
                    cx.notify();
                }
            }))
            .child(self.render_topbar(cx))
            .when(self.error.is_some(), |d| d.child(self.render_error(cx)))
            .child(if self.user.is_none() {
                self.render_connect(cx)
            } else {
                h_flex()
                    .items_stretch()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(self.render_rail(cx))
                    .when(self.sidebar_visible, |d| d.child(self.render_sidebar(cx)))
                    .child(v_flex().flex_1().min_w_0().h_full().child(match self.page {
                        Page::Home => self.render_home(cx),
                        Page::Inbox => self.render_inbox(cx),
                        Page::MyTasks | Page::Project(_) => self.render_task_page(cx),
                        Page::Projects => self.render_projects(cx),
                        Page::Settings => self.render_settings(cx),
                    }))
                    .when(self.selected.is_some(), |d| {
                        d.child(self.render_task_detail(cx))
                    })
                    .into_any_element()
            })
            .child(self.render_status(cx))
    }
}
