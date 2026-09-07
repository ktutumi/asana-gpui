use super::*;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        input::{Input, Textarea},
        menu::{DropdownMenu, PopupMenuItem},
    },
    prelude::FluentBuilder,
};

impl WorkspaceApp {
    pub(super) fn render_task_detail(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(task) = &self.selected else {
            return div().into_any_element();
        };
        let toggle = task.clone();
        let discard = task.clone();
        let weak = cx.weak_entity();
        let users = self.data.users.clone();
        let assignee = self.assignee.clone();
        let gid = task.gid.clone();
        let permalink = task.permalink_url.clone();
        let workspace = self.data.workspace.gid.clone();
        v_flex()
            .size_full()
            .flex_shrink_0()
            .border_l_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .px_5()
                    .py_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("detail-complete")
                            .small()
                            .icon(IconName::CircleCheck)
                            .label(if task.completed {
                                "Completed"
                            } else {
                                "Mark complete"
                            })
                            .selected(task.completed)
                            .disabled(self.busy.is_some())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.toggle_task(toggle.clone(), window, cx)
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("open-asana")
                            .ghost()
                            .small()
                            .icon(IconName::ExternalLink)
                            .accessibility_label("Open in Asana")
                            .tooltip("Open in Asana")
                            .disabled(self.demo)
                            .on_click(move |_, _, cx| {
                                let url = if permalink.is_empty() {
                                    format!("https://app.asana.com/1/{workspace}/task/{gid}")
                                } else {
                                    permalink.clone()
                                };
                                if url::Url::parse(&url).is_ok_and(|u| {
                                    u.scheme() == "https" && u.host_str() == Some("app.asana.com")
                                }) {
                                    cx.open_url(&url);
                                }
                            }),
                    )
                    .child(
                        Button::new("close-detail")
                            .ghost()
                            .small()
                            .icon(IconName::Close)
                            .accessibility_label("Close task · Esc")
                            .tooltip("Close task · Esc")
                            .on_click(cx.listener(|this, _, window, cx| {
                                if this.can_leave(cx) {
                                    this.selected = None;
                                    this.focus.focus(window, cx);
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .child(
                v_flex()
                    .id("task-detail-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_5()
                    .gap_5()
                    .child(
                        self.field(
                            "Task name",
                            Input::new(&self.title)
                                .large()
                                .aria_label("Task name")
                                .disabled(self.busy.is_some()),
                            cx,
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .items_start()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Assignee"),
                                    )
                                    .child(
                                        Button::new("assignee-select")
                                            .ghost()
                                            .justify_start()
                                            .icon(IconName::CircleUser)
                                            .label(
                                                self.assignee
                                                    .as_ref()
                                                    .map(|u| u.name.clone())
                                                    .unwrap_or_else(|| "Unassigned".into()),
                                            )
                                            .disabled(self.busy.is_some())
                                            .dropdown_menu(move |mut menu, _, _| {
                                                let weak2 = weak.clone();
                                                menu = menu.scrollable(true).item(
                                                    PopupMenuItem::new("Unassigned")
                                                        .checked(assignee.is_none())
                                                        .on_click(move |_, _, cx| {
                                                            let _ = weak2.update(cx, |this, cx| {
                                                                this.assignee = None;
                                                                cx.notify();
                                                            });
                                                        }),
                                                );
                                                for user in &users {
                                                    let weak = weak.clone();
                                                    let user = user.clone();
                                                    menu = menu.item(
                                                        PopupMenuItem::new(user.name.clone())
                                                            .checked(
                                                                assignee.as_ref().is_some_and(
                                                                    |a| a.gid == user.gid,
                                                                ),
                                                            )
                                                            .on_click(move |_, _, cx| {
                                                                let _ =
                                                                    weak.update(cx, |this, cx| {
                                                                        this.assignee =
                                                                            Some(user.clone());
                                                                        cx.notify();
                                                                    });
                                                            }),
                                                    );
                                                }
                                                menu
                                            }),
                                    ),
                            )
                            .child(
                                div().w(rems(9.)).child(
                                    self.field(
                                        "Due date",
                                        Input::new(&self.due)
                                            .aria_label("Due date YYYY-MM-DD")
                                            .disabled(self.busy.is_some()),
                                        cx,
                                    ),
                                ),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Projects"),
                            )
                            .children(task.projects.iter().map(|p| {
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Icon::new(IconName::Folder)
                                            .small()
                                            .text_color(cx.theme().primary),
                                    )
                                    .child(div().text_sm().child(p.name.clone()))
                            })),
                    )
                    .when(matches!(self.page, Page::Project(_)), |d| {
                        let sections = self
                            .project
                            .as_ref()
                            .map(|p| p.sections.clone())
                            .unwrap_or_default();
                        let weak = cx.weak_entity();
                        let project = match &self.page {
                            Page::Project(gid) => Some(gid.as_str()),
                            _ => None,
                        };
                        d.child(
                            v_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Section"),
                                )
                                .child(
                                    Button::new("section-select")
                                        .ghost()
                                        .justify_start()
                                        .label(task.section(project).name)
                                        .icon(IconName::ChevronDown)
                                        .disabled(self.busy.is_some())
                                        .dropdown_menu(move |mut menu, _, _| {
                                            for section in &sections {
                                                let weak = weak.clone();
                                                let section = section.clone();
                                                menu = menu.item(
                                                    PopupMenuItem::new(section.name.clone())
                                                        .on_click(move |_, window, cx| {
                                                            let _ = weak.update(cx, |this, cx| {
                                                                this.move_section(
                                                                    section.clone(),
                                                                    window,
                                                                    cx,
                                                                )
                                                            });
                                                        }),
                                                );
                                            }
                                            menu
                                        }),
                                ),
                        )
                    })
                    .child(
                        self.field(
                            "Description",
                            Textarea::new(&self.description)
                                .h(rems(12.))
                                .aria_label("Task description")
                                .disabled(self.busy.is_some()),
                            cx,
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("save-task")
                                    .primary()
                                    .label("Save changes")
                                    .disabled(self.busy.is_some() || !self.dirty(cx))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save_task(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("discard-task")
                                    .ghost()
                                    .label("Discard changes")
                                    .disabled(self.busy.is_some() || !self.dirty(cx))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.set_editor(discard.clone(), window, cx);
                                        this.error = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .when(!self.subtasks.is_empty(), |d| {
                        d.child(
                            v_flex()
                                .gap_3()
                                .child(div().font_semibold().child("Subtasks"))
                                .children(self.subtasks.iter().map(|t| self.compact_task(t, cx))),
                        )
                    })
                    .child(
                        div()
                            .pt_3()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .font_semibold()
                            .child("Activity"),
                    )
                    .when(
                        !self
                            .stories
                            .iter()
                            .any(|story| story.resource_subtype == "comment_added"),
                        |d| {
                            d.child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("No activity to show yet."),
                            )
                        },
                    )
                    .children(
                        self.stories
                            .iter()
                            .filter(|s| s.resource_subtype == "comment_added")
                            .map(|story| {
                                v_flex()
                                    .gap_2()
                                    .p_4()
                                    .rounded_lg()
                                    .bg(cx.theme().sidebar)
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                div().text_sm().font_semibold().child(
                                                    story
                                                        .created_by
                                                        .as_ref()
                                                        .map(|u| u.name.clone())
                                                        .unwrap_or_else(|| "Asana".into()),
                                                ),
                                            )
                                            .child(div().flex_1())
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(
                                                        story
                                                            .created_at
                                                            .map(|d| {
                                                                d.with_timezone(&Local)
                                                                    .format("%b %-d, %H:%M")
                                                                    .to_string()
                                                            })
                                                            .unwrap_or_default(),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .whitespace_normal()
                                            .child(story.text.clone()),
                                    )
                            }),
                    )
                    .child(
                        self.field(
                            "Comment",
                            Textarea::new(&self.comment)
                                .h(rems(6.))
                                .aria_label("Write a comment")
                                .disabled(self.busy.is_some()),
                            cx,
                        ),
                    )
                    .child(
                        h_flex().justify_end().child(
                            Button::new("post-comment")
                                .label("Post comment")
                                .disabled(
                                    self.busy.is_some()
                                        || self.comment.read(cx).value().trim().is_empty(),
                                )
                                .on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.post_comment(window, cx)
                                    }),
                                ),
                        ),
                    ),
            )
            .into_any_element()
    }
}
