use crate::{
    model::*,
    oauth::{self, Session},
};
use anyhow::{Context, Result, bail, ensure};
use reqwest::{Method, StatusCode, blocking::Client};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

const TASK_FIELDS: &str = "gid,name,completed,completed_at,due_on,due_at,assignee.name,assignee_section.name,notes,projects.name,memberships.project.name,memberships.section.name,modified_at,num_subtasks,permalink_url";
const PROJECT_FIELDS: &str = "gid,name,notes,color,due_on,owner.name,current_status.title,current_status.text,current_status.color";

#[derive(Clone)]
pub struct AsanaClient {
    http: Client,
    base: String,
    session: Arc<Mutex<Session>>,
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
    next_page: Option<NextPage>,
}
#[derive(Deserialize)]
struct NextPage {
    offset: String,
}

impl AsanaClient {
    pub fn new(session: Session) -> Result<Self> {
        Ok(Self {
            http: oauth::http_client()?,
            base: "https://app.asana.com/api/1.0".into(),
            session: Arc::new(Mutex::new(session)),
        })
    }

    pub fn credentials(&self) -> Result<Vec<u8>> {
        let session = self
            .session
            .lock()
            .map_err(|_| anyhow::anyhow!("The session is unavailable. Reconnect to Asana."))?;
        serde_json::to_vec(&*session).context("Could not encode the session.")
    }

    pub fn is_oauth(&self) -> bool {
        self.session
            .lock()
            .is_ok_and(|s| !s.refresh_token.is_empty())
    }

    fn token(&self, force: bool) -> Result<String> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| anyhow::anyhow!("The session is unavailable. Reconnect to Asana."))?;
        // Serialize refreshes so concurrent requests never rotate the same refresh token twice.
        if session.needs_refresh() || force {
            session.refresh(&self.http)?;
        }
        Ok(session.access_token.clone())
    }

    fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<T> {
        let mut token = self.token(false)?;
        let mut refreshed = false;
        let mut rate_retries = 0;
        loop {
            let mut request = self
                .http
                .request(method.clone(), format!("{}{path}", self.base))
                .bearer_auth(&token)
                .query(query);
            if let Some(body) = body {
                request = request.json(&json!({"data":body}));
            }
            let response = request.send().map_err(|_| anyhow::anyhow!("Could not reach Asana. Check your connection. For an interrupted save, refresh before retrying."))?;
            if response.status() == StatusCode::UNAUTHORIZED && !refreshed && self.is_oauth() {
                token = self.token(true)?;
                refreshed = true;
                continue;
            }
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                let wait = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(5);
                if method == Method::GET && rate_retries < 2 && wait <= 10 {
                    rate_retries += 1;
                    std::thread::sleep(Duration::from_secs(wait));
                    continue;
                }
                bail!("Asana's request limit was reached. Wait {wait} seconds, then refresh.");
            }
            match response.status() {
                StatusCode::UNAUTHORIZED => {
                    bail!("Your Asana session is no longer valid. Connect again.")
                }
                StatusCode::FORBIDDEN => bail!(
                    "Asana denied access. Check this item's permissions and the OAuth app's scopes."
                ),
                StatusCode::NOT_FOUND => {
                    bail!("This item is no longer available in Asana. Refresh the workspace.")
                }
                status if !status.is_success() => {
                    // Do not echo response bodies: upstream errors can contain submitted fields.
                    bail!(
                        "Asana could not complete the request (HTTP {}). Check the fields and refresh before retrying a save.",
                        status.as_u16()
                    );
                }
                _ => {}
            }
            return response.json().map_err(|_| {
                anyhow::anyhow!(
                    "Asana returned an unreadable response. Refresh to check the current state."
                )
            });
        }
    }

    fn list<T: DeserializeOwned>(
        &self,
        path: &str,
        mut query: Vec<(&str, String)>,
    ) -> Result<Vec<T>> {
        query.push(("limit", "100".into()));
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        loop {
            let page: Envelope<Vec<T>> = self.request(Method::GET, path, &query, None)?;
            items.extend(page.data);
            let Some(next) = page.next_page.filter(|p| !p.offset.is_empty()) else {
                return Ok(items);
            };
            ensure!(
                seen.insert(next.offset.clone()),
                "Asana repeated a page cursor. Refresh to retry loading the full list."
            );
            query.retain(|(key, _)| *key != "offset");
            // Only use the opaque offset. Never follow a server-provided URL with credentials.
            query.push(("offset", next.offset));
        }
    }

    fn one<T: DeserializeOwned>(&self, path: &str, fields: &str) -> Result<T> {
        let result: Envelope<T> =
            self.request(Method::GET, path, &[("opt_fields", fields.into())], None)?;
        Ok(result.data)
    }

    pub fn identity(&self) -> Result<(User, Vec<Named>)> {
        Ok((
            self.one("/users/me", "gid,name")?,
            self.list("/workspaces", vec![("opt_fields", "gid,name".into())])?,
        ))
    }

    pub fn workspace(&self, workspace: Named) -> Result<WorkspaceData> {
        validate_gid(&workspace.gid)?;
        let projects = self.list(
            "/projects",
            vec![
                ("workspace", workspace.gid.clone()),
                ("archived", "false".into()),
                ("opt_fields", PROJECT_FIELDS.into()),
            ],
        )?;
        let tasks = self.list(
            "/tasks",
            vec![
                ("workspace", workspace.gid.clone()),
                ("assignee", "me".into()),
                ("completed_since", "1970-01-01T00:00:00Z".into()),
                ("opt_fields", TASK_FIELDS.into()),
            ],
        )?;
        let users = self.list(
            &format!("/workspaces/{}/users", workspace.gid),
            vec![("opt_fields", "gid,name".into())],
        )?;
        Ok(WorkspaceData {
            workspace,
            projects,
            tasks,
            users,
        })
    }

    pub fn project(&self, gid: &str) -> Result<ProjectData> {
        validate_gid(gid)?;
        Ok(ProjectData {
            project: self.one(&format!("/projects/{gid}"), PROJECT_FIELDS)?,
            sections: self.list(
                &format!("/projects/{gid}/sections"),
                vec![("opt_fields", "gid,name".into())],
            )?,
            tasks: self.list(
                &format!("/projects/{gid}/tasks"),
                vec![
                    ("completed_since", "1970-01-01T00:00:00Z".into()),
                    ("opt_fields", TASK_FIELDS.into()),
                ],
            )?,
        })
    }

    pub fn task(&self, gid: &str) -> Result<Task> {
        validate_gid(gid)?;
        self.one(&format!("/tasks/{gid}"), TASK_FIELDS)
    }

    pub fn stories(&self, gid: &str) -> Result<Vec<Story>> {
        validate_gid(gid)?;
        self.list(
            &format!("/tasks/{gid}/stories"),
            vec![(
                "opt_fields",
                "gid,text,resource_subtype,created_at,created_by.name".into(),
            )],
        )
    }

    pub fn subtasks(&self, gid: &str) -> Result<Vec<Task>> {
        validate_gid(gid)?;
        self.list(
            &format!("/tasks/{gid}/subtasks"),
            vec![("opt_fields", TASK_FIELDS.into())],
        )
    }

    pub fn update_task(&self, gid: &str, patch: Value) -> Result<Task> {
        validate_gid(gid)?;
        let result: Envelope<Task> = self.request(
            Method::PUT,
            &format!("/tasks/{gid}"),
            &[("opt_fields", TASK_FIELDS.into())],
            Some(&patch),
        )?;
        Ok(result.data)
    }

    pub fn create_task(&self, workspace: &str, project: Option<&str>, name: &str) -> Result<Task> {
        validate_gid(workspace)?;
        ensure!(!name.trim().is_empty(), "Enter a task name.");
        let mut body = json!({"workspace":workspace,"name":name.trim(),"assignee":"me"});
        if let Some(project) = project {
            validate_gid(project)?;
            body["projects"] = json!([project]);
        }
        let result: Envelope<Task> = self.request(
            Method::POST,
            "/tasks",
            &[("opt_fields", TASK_FIELDS.into())],
            Some(&body),
        )?;
        Ok(result.data)
    }

    pub fn comment(&self, gid: &str, text: &str) -> Result<Story> {
        validate_gid(gid)?;
        ensure!(!text.trim().is_empty(), "Enter a comment.");
        let result: Envelope<Story> = self.request(
            Method::POST,
            &format!("/tasks/{gid}/stories"),
            &[(
                "opt_fields",
                "gid,text,resource_subtype,created_at,created_by.name".into(),
            )],
            Some(&json!({"text":text.trim()})),
        )?;
        Ok(result.data)
    }

    pub fn move_to_section(&self, gid: &str, section: &str) -> Result<Task> {
        validate_gid(gid)?;
        validate_gid(section)?;
        let _: Value = self.request(
            Method::POST,
            &format!("/sections/{section}/addTask"),
            &[],
            Some(&json!({"task":gid})),
        )?;
        self.task(gid)
    }

    pub fn create_project(&self, workspace: &str, name: &str) -> Result<Project> {
        validate_gid(workspace)?;
        ensure!(!name.trim().is_empty(), "Enter a project name.");
        let result:Envelope<Project>=self.request(Method::POST,"/projects",&[("opt_fields",PROJECT_FIELDS.into())],Some(&json!({"workspace":workspace,"name":name.trim(),"privacy_setting":"private","default_view":"list","color":"light-blue"})))?;
        Ok(result.data)
    }
}

fn validate_gid(gid: &str) -> Result<()> {
    ensure!(
        !gid.is_empty() && gid.bytes().all(|b| b.is_ascii_digit()),
        "The Asana item ID is invalid."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };

    #[test]
    fn pagination_keeps_query_and_ignores_next_page_uri() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for page in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = [0u8; 8192];
                let n = stream.read(&mut bytes).unwrap();
                let req = String::from_utf8_lossy(&bytes[..n]);
                assert!(req.contains("limit=100") && req.contains("archived=false"));
                assert!(
                    req.to_lowercase()
                        .contains("authorization: bearer test-token")
                );
                if page == 1 {
                    assert!(req.contains("offset=page2"));
                }
                let body = if page == 0 {
                    r#"{"data":[{"gid":"1","name":"one"}],"next_page":{"offset":"page2","uri":"https://untrusted.invalid"}}"#
                } else {
                    r#"{"data":[{"gid":"2","name":"two"}],"next_page":null}"#
                };
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let mut client = AsanaClient::new(Session {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: oauth::DEFAULT_REDIRECT.into(),
            access_token: "test-token".into(),
            refresh_token: String::new(),
            expires_at: None,
        })
        .unwrap();
        client.base = format!("http://{address}");
        let items: Vec<Named> = client
            .list("/projects", vec![("archived", "false".into())])
            .unwrap();
        assert_eq!(
            items.iter().map(|p| p.gid.as_str()).collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        server.join().unwrap();
        assert!(validate_gid("1/../users/me").is_err());
    }

    #[test]
    #[ignore = "Explicitly opt in with a Test workspace, test project, and ASANA_API_KEY"]
    fn live_test_workspace_roundtrip() {
        let workspace_gid =
            std::env::var("ASANA_GPUI_TEST_WORKSPACE").expect("Set the Test workspace GID");
        let project_gid = std::env::var("ASANA_GPUI_TEST_PROJECT").expect("Set a test project GID");
        let api = AsanaClient::new(Session::from_environment_token().unwrap()).unwrap();
        let (user, workspaces) = api.identity().unwrap();
        let workspace = workspaces
            .into_iter()
            .find(|w| w.gid == workspace_gid)
            .expect("Workspace not found");
        assert_eq!(workspace.name, "Test", "Only the Test workspace is allowed");
        let data = api.workspace(workspace).unwrap();
        assert!(
            data.projects
                .iter()
                .any(|p| p.gid == project_gid && p.name.starts_with("GPUI Client"))
        );
        let project = api.project(&project_gid).unwrap();
        let task = api
            .create_task(
                &workspace_gid,
                Some(&project_gid),
                "GPUI API verification — create, update, complete",
            )
            .unwrap();
        assert_eq!(task.assignee.as_ref().unwrap().gid, user.gid);
        let task=api.update_task(&task.gid,json!({"notes":"Created by the explicitly enabled GPUI API integration test.","due_on":"2026-09-10"})).unwrap();
        assert_eq!(task.due_on.unwrap().to_string(), "2026-09-10");
        let story = api
            .comment(
                &task.gid,
                "GPUI integration test: comment round trip verified.",
            )
            .unwrap();
        assert!(
            api.stories(&task.gid)
                .unwrap()
                .iter()
                .any(|s| s.gid == story.gid)
        );
        if let Some(section) = project.sections.first() {
            let task = api.move_to_section(&task.gid, &section.gid).unwrap();
            assert!(
                task.memberships
                    .iter()
                    .any(|m| m.section.as_ref().is_some_and(|s| s.gid == section.gid))
            );
        }
        let task = api
            .update_task(&task.gid, json!({"completed":true}))
            .unwrap();
        assert!(task.completed);
        assert!(api.task(&task.gid).unwrap().completed);
        assert!(api.subtasks(&task.gid).unwrap().is_empty());
        println!(
            "Verified Test workspace, project, assigned tasks, task creation/update/completion, sections, comments, and subtasks."
        );
    }
}
