use std::{ops::Deref, str::FromStr};

use either::Either;
use octocrab::models::{self, activity::Subject};
use ratatui::{
    style::{Color, Stylize},
    text::Span,
};
use synd_feed::types::Category;
use url::Url;

use crate::{
    client::github::{issue_query, pull_request_query},
    config::Categories,
    types::Time,
    ui::{self, icon},
};

pub(crate) type ThreadId = octocrab::models::ThreadId;

pub(crate) type NotificationId = octocrab::models::NotificationId;

#[derive(Debug, Clone)]
pub(crate) struct IssueId(i64);

impl IssueId {
    pub(crate) fn into_inner(self) -> i64 {
        self.0
    }
}

impl Deref for IssueId {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PullRequestId(i64);

impl PullRequestId {
    pub(crate) fn into_inner(self) -> i64 {
        self.0
    }
}

impl Deref for PullRequestId {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepoVisibility {
    Public,
    Private,
}

#[derive(Debug, Clone)]
pub(crate) struct RepositoryKey {
    pub(crate) name: String,
    pub(crate) owner: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Repository {
    pub(crate) name: String,
    pub(crate) owner: String,
    pub(crate) visibility: RepoVisibility,
}

#[derive(Debug, Clone)]
pub(crate) struct NotificationContext<ID> {
    pub(crate) id: ID,
    pub(crate) notification_id: NotificationId,
    pub(crate) repository_key: RepositoryKey,
}

pub(crate) type IssueOrPullRequest =
    Either<NotificationContext<IssueId>, NotificationContext<PullRequestId>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SubjectType {
    Issue,
    PullRequest,
    Ci,
    Release,
    Discussion,
}

/// Additional information fetched from api
#[derive(Debug, Clone)]
pub(crate) enum SubjectContext {
    Issue(IssueContext),
    PullRequest(PullRequestContext),
}

/// `https://docs.github.com/en/rest/activity/notifications?apiVersion=2022-11-28#about-notification-reasons`
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reason {
    Assign,
    Author,
    CiActivity,
    ManuallySubscribed,
    Mention,
    TeamMention,
    ReviewRequested,
    WatchingRepo,
    Other(String),
}

#[derive(Clone, Debug)]
pub(crate) struct Notification {
    pub(crate) id: NotificationId,
    pub(crate) thread_id: Option<ThreadId>,
    pub(crate) reason: Reason,
    #[allow(unused)]
    pub(crate) unread: bool,
    pub(crate) updated_at: Time,
    pub(crate) last_read_at: Option<Time>,
    pub(crate) repository: Repository,
    pub(crate) subject_context: Option<SubjectContext>,
    categories: Vec<Category<'static>>,
    subject_type: Option<SubjectType>,
    subject: Subject,
}

impl From<models::activity::Notification> for Notification {
    fn from(
        models::activity::Notification {
            id,
            repository,
            subject,
            reason,
            unread,
            updated_at,
            last_read_at,
            url,
            ..
        }: models::activity::Notification,
    ) -> Self {
        let (owner, name) = if let Some(full_name) = repository.full_name {
            let mut s = full_name.splitn(2, '/');
            if let (Some(owner), Some(repo)) = (s.next(), s.next()) {
                (owner.to_owned(), repo.to_owned())
            } else {
                tracing::warn!("Unexpected repository full_name: `{full_name}`");
                (String::new(), repository.name)
            }
        } else {
            tracing::warn!("Repository full_name not found");
            (String::new(), repository.name)
        };
        let repository = Repository {
            name,
            owner,
            visibility: if repository.private.unwrap_or(false) {
                RepoVisibility::Private
            } else {
                RepoVisibility::Public
            },
        };

        // Assume url is like "https://api.github.com/notifications/threads/11122223333"
        let thread_id = url
            .path_segments()
            .and_then(|mut seg| seg.nth(2))
            .and_then(|id| id.parse::<u64>().ok())
            .map(ThreadId::from);

        let categories = vec![ui::default_category().clone()];

        let subject_type = match subject.r#type.as_str() {
            typ if typ.eq_ignore_ascii_case("issue") => Some(SubjectType::Issue),
            typ if typ.eq_ignore_ascii_case("pullrequest") => Some(SubjectType::PullRequest),
            typ if typ.eq_ignore_ascii_case("checksuite") && reason == "ci_activity" => {
                Some(SubjectType::Ci)
            }
            typ if typ.eq_ignore_ascii_case("release") => Some(SubjectType::Release),
            typ if typ.eq_ignore_ascii_case("discussion") => Some(SubjectType::Discussion),
            _ => {
                tracing::warn!("Unknown url: {url:?} reason: {reason} subject: `{subject:?}`");
                None
            }
        };

        let reason = match reason.as_str() {
            "assign" => Reason::Assign,
            "author" => Reason::Author,
            "ci_activity" => Reason::CiActivity,
            "manual" => Reason::ManuallySubscribed,
            "mention" => Reason::Mention,
            "team_mention" => Reason::TeamMention,
            "review_requested" => Reason::ReviewRequested,
            "subscribed" => Reason::WatchingRepo,
            other => Reason::Other(other.to_owned()),
        };

        Self {
            id,
            thread_id,
            reason,
            unread,
            updated_at,
            last_read_at,
            repository,
            categories,
            subject,
            subject_type,
            subject_context: None,
        }
    }
}

impl Notification {
    pub(crate) fn subject_type(&self) -> Option<SubjectType> {
        self.subject_type
    }

    pub(crate) fn subject_icon(&self) -> Span {
        match self.subject_type() {
            Some(SubjectType::Issue) => match self.subject_context {
                Some(SubjectContext::Issue(ref issue)) => match issue.state {
                    IssueState::Open => {
                        if matches!(issue.state_reason, Some(IssueStateReason::ReOpened)) {
                            Span::from(icon!(issuereopened)).green()
                        } else {
                            Span::from(icon!(issueopen)).green()
                        }
                    }
                    IssueState::Closed => {
                        if matches!(issue.state_reason, Some(IssueStateReason::NotPlanned)) {
                            Span::from(icon!(issuenotplanned)).gray()
                        } else {
                            Span::from(icon!(issueclosed)).light_magenta()
                        }
                    }
                },
                _ => Span::from(icon!(issueopen)),
            },
            Some(SubjectType::PullRequest) => match self.subject_context {
                Some(SubjectContext::PullRequest(ref pr)) => match pr.state {
                    PullRequestState::Open => {
                        if pr.is_draft {
                            Span::from(icon!(pullrequestdraft)).gray()
                        } else {
                            Span::from(icon!(pullrequest)).green()
                        }
                    }
                    PullRequestState::Merged => {
                        Span::from(icon!(pullrequestmerged)).light_magenta()
                    }
                    PullRequestState::Closed => Span::from(icon!(pullrequestclosed)).red(),
                },
                _ => Span::from(icon!(pullrequest)),
            },
            Some(SubjectType::Ci) => Span::from(icon!(cross)).red(),
            Some(SubjectType::Release) => Span::from(icon!(tag)).green(),
            Some(SubjectType::Discussion) => Span::from(icon!(discussion)),
            None => Span::from(" "),
        }
    }

    pub(crate) fn title(&self) -> &str {
        &self.subject.title
    }

    pub(crate) fn browser_url(&self) -> Option<Url> {
        let mut url = self.base_url();
        match self.subject_type()? {
            SubjectType::Issue => {
                // construct like "https://github.com/ymgyt/syndicationd/issues/{issue-id}#issumecomment-{commentid}"

                url.path_segments_mut()
                    .unwrap()
                    .extend(["issues", &self.issue_id()?.to_string()]);
                if let Some(commend_id) = self.comment_id() {
                    url.set_fragment(Some(&format!("issuecomment-{commend_id}")));
                }
                Some(url)
            }
            SubjectType::PullRequest => {
                // construct like "https://github.com/ymgyt/syndicationd/pull/{pr-id}#pullrequestreview-123"
                url.path_segments_mut()
                    .unwrap()
                    .extend(["pull", &self.pull_request_id()?.to_string()]);

                // How to get PR review comment id?
                Some(url)
            }
            SubjectType::Ci => {
                // In th UI, it transitions to the failed actions
                // but I don't know how to identify which action failed
                url.path_segments_mut().unwrap().extend(["actions"]);
                Some(url)
            }
            SubjectType::Release => {
                // Since the release ID is stored in the subject.url, obtaining the release information might help determine the specific destination
                url.path_segments_mut().unwrap().extend(["releases"]);
                Some(url)
            }
            SubjectType::Discussion => {
                url.path_segments_mut().unwrap().extend(["discussions"]);
                Some(url)
            }
        }
    }

    pub(crate) fn context(&self) -> Option<IssueOrPullRequest> {
        match self.subject_type()? {
            SubjectType::Issue => Some(Either::Left(NotificationContext {
                id: self.issue_id()?,
                notification_id: self.id,
                repository_key: self.repository_key().clone(),
            })),
            SubjectType::PullRequest => Some(Either::Right(NotificationContext {
                id: self.pull_request_id()?,
                notification_id: self.id,
                repository_key: self.repository_key().clone(),
            })),
            // Currently ignore ci, release, discussion
            _ => None,
        }
    }

    pub(crate) fn author(&self) -> Option<String> {
        match self.subject_context {
            Some(SubjectContext::Issue(ref issue)) => issue.author.clone(),
            Some(SubjectContext::PullRequest(ref pr)) => pr.author.clone(),
            _ => None,
        }
    }

    pub(crate) fn body(&self) -> Option<String> {
        match self.subject_context {
            Some(SubjectContext::Issue(ref issue)) => Some(issue.body.clone()),
            Some(SubjectContext::PullRequest(ref pr)) => Some(pr.body.clone()),
            _ => None,
        }
    }

    pub(crate) fn last_comment(&self) -> Option<Comment> {
        match self.subject_context {
            Some(SubjectContext::Issue(ref issue)) => issue.last_comment.clone(),
            Some(SubjectContext::PullRequest(ref pr)) => pr.last_comment.clone(),
            _ => None,
        }
    }

    pub(crate) fn issue_id(&self) -> Option<IssueId> {
        // Assume url is like "https://api.github.com/repos/ymgyt/synd/issues/123"
        let mut segments = self.subject.url.as_ref()?.path_segments()?.skip(3);
        (segments.next() == Some("issues"))
            .then(|| segments.next())?
            .and_then(|id| id.parse().ok())
            .map(IssueId)
    }

    pub(crate) fn pull_request_id(&self) -> Option<PullRequestId> {
        // Assume url is like "https://api.github.com/repos/ymgyt/synd/pulls/123"
        let mut segments = self.subject.url.as_ref()?.path_segments()?.skip(3);
        (segments.next() == Some("pulls"))
            .then(|| segments.next())?
            .and_then(|id| id.parse().ok())
            .map(PullRequestId)
    }

    fn repository_key(&self) -> RepositoryKey {
        RepositoryKey {
            name: self.repository.name.clone(),
            owner: self.repository.owner.clone(),
        }
    }

    fn comment_id(&self) -> Option<String> {
        // Assume url is like "https://api.github.com/repos/ymgyt/synd/issues/comments/123"
        let mut segments = self
            .subject
            .latest_comment_url
            .as_ref()?
            .path_segments()?
            .skip(4);
        (segments.next() == Some("comments"))
            .then(|| segments.next())?
            .map(ToString::to_string)
    }

    // Return https://github.com/{owner}/{repo}
    fn base_url(&self) -> Url {
        let mut url = Url::parse("https://github.com").unwrap();
        url.path_segments_mut().unwrap().extend([
            self.repository.owner.as_str(),
            self.repository.name.as_str(),
        ]);

        url
    }

    pub(crate) fn categories(&self) -> impl Iterator<Item = &Category<'static>> {
        self.categories.iter()
    }

    pub(crate) fn update_categories(&mut self, config: &Categories) {
        self.categories.clear();
        if let Some(category) = config.lookup(&self.repository.owner) {
            self.categories.push(category);
        }
        if let Some(category) = config.lookup(&self.repository.name) {
            self.categories.push(category);
        }
        if let Some(topics) = self.topics().map(|topics| {
            topics
                .filter_map(|topic| config.lookup(topic))
                .collect::<Vec<_>>()
        }) {
            self.categories.extend(topics);
        }
        if self.categories.is_empty() {
            self.categories.push(ui::default_category().clone());
        }
    }

    fn topics(&self) -> Option<impl Iterator<Item = &str>> {
        match self.subject_context {
            Some(SubjectContext::Issue(ref issue)) => Some(issue.topics.iter().map(String::as_str)),
            Some(SubjectContext::PullRequest(ref pr)) => Some(pr.topics.iter().map(String::as_str)),
            _ => None,
        }
    }

    pub(crate) fn labels(&self) -> Option<impl Iterator<Item = &Label>> {
        match self.subject_context {
            Some(SubjectContext::Issue(ref issue)) => {
                if issue.labels.is_empty() {
                    None
                } else {
                    Some(issue.labels.iter())
                }
            }
            Some(SubjectContext::PullRequest(ref pr)) => {
                if pr.labels.is_empty() {
                    None
                } else {
                    Some(pr.labels.iter())
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Comment {
    pub(crate) author: String,
    pub(crate) body: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Label {
    pub(crate) name: String,
    pub(crate) color: Option<Color>,
    pub(crate) luminance: Option<f64>,
}

#[derive(Debug, Clone)]
pub(crate) enum IssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone)]
pub(crate) enum IssueStateReason {
    ReOpened,
    NotPlanned,
    Completed,
}

#[derive(Debug, Clone)]
pub(crate) struct IssueContext {
    author: Option<String>,
    #[allow(unused)]
    topics: Vec<String>,
    state: IssueState,
    state_reason: Option<IssueStateReason>,
    body: String,
    last_comment: Option<Comment>,
    labels: Vec<Label>,
}

impl From<issue_query::ResponseData> for IssueContext {
    fn from(data: issue_query::ResponseData) -> Self {
        let repo = data
            .repository
            .expect("ResponseData does not have repository");
        let topics: Vec<String> = repo
            .repository_topics
            .nodes
            .unwrap_or_default()
            .into_iter()
            .filter_map(|node| node.map(|node| node.topic.name))
            .collect();
        let issue = repo.issue.expect("ResponseData does not have issue");
        let author: Option<String> = issue.author.map(|author| author.login);
        let state = match issue.state {
            issue_query::IssueState::OPEN | issue_query::IssueState::Other(_) => IssueState::Open,
            issue_query::IssueState::CLOSED => IssueState::Closed,
        };
        let state_reason = match issue.state_reason {
            Some(issue_query::IssueStateReason::REOPENED) => Some(IssueStateReason::ReOpened),
            Some(issue_query::IssueStateReason::NOT_PLANNED) => Some(IssueStateReason::NotPlanned),
            Some(issue_query::IssueStateReason::COMPLETED) => Some(IssueStateReason::Completed),
            _ => None,
        };
        let body = issue.body_text;
        let last_comment: Option<Comment> = issue
            .comments
            .nodes
            .unwrap_or_default()
            .into_iter()
            .find_map(|node| {
                node.map(|node| Comment {
                    author: node.author.map(|author| author.login).unwrap_or_default(),
                    body: node.body_text,
                })
            });
        let labels = issue
            .labels
            .and_then(|labels| labels.nodes)
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(|label| Label {
                name: label.name,
                color: Color::from_str(&format!("#{}", label.color)).ok(),
                luminance: luminance(&label.color),
            })
            .collect();

        Self {
            author,
            topics,
            state,
            state_reason,
            body,
            last_comment,
            labels,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PullRequestState {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone)]
pub(crate) struct PullRequestContext {
    author: Option<String>,
    #[allow(unused)]
    topics: Vec<String>,
    pub(crate) state: PullRequestState,
    is_draft: bool,
    body: String,
    last_comment: Option<Comment>,
    labels: Vec<Label>,
}

impl From<pull_request_query::ResponseData> for PullRequestContext {
    fn from(data: pull_request_query::ResponseData) -> Self {
        let repo = data
            .repository
            .expect("ResponseData does not have repository");

        let topics: Vec<String> = repo
            .repository_topics
            .nodes
            .unwrap_or_default()
            .into_iter()
            .filter_map(|node| node.map(|node| node.topic.name))
            .collect();

        let pr = repo
            .pull_request
            .expect("ResponseData does not have pull request");
        let author: Option<String> = pr.author.map(|author| author.login);
        let state = match pr.state {
            pull_request_query::PullRequestState::OPEN
            | pull_request_query::PullRequestState::Other(_) => PullRequestState::Open,
            pull_request_query::PullRequestState::CLOSED => PullRequestState::Closed,
            pull_request_query::PullRequestState::MERGED => PullRequestState::Merged,
        };
        let is_draft = pr.is_draft;
        let body = pr.body_text;
        let last_comment: Option<Comment> = pr
            .comments
            .nodes
            .unwrap_or_default()
            .into_iter()
            .find_map(|node| {
                node.map(|node| Comment {
                    author: node.author.map(|author| author.login).unwrap_or_default(),
                    body: node.body_text,
                })
            });
        let labels = pr
            .labels
            .and_then(|labels| labels.nodes)
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(|label| Label {
                name: label.name,
                color: Color::from_str(&format!("#{}", label.color)).ok(),
                luminance: luminance(&label.color),
            })
            .collect();

        Self {
            author,
            topics,
            state,
            is_draft,
            body,
            last_comment,
            labels,
        }
    }
}

// Assume color is "RRGGBB" in hex format
#[allow(clippy::cast_lossless)]
fn luminance(color: &str) -> Option<f64> {
    if color.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&color[..2], 16).ok()? as f64;
    let g = u8::from_str_radix(&color[2..4], 16).ok()? as f64;
    let b = u8::from_str_radix(&color[4..], 16).ok()? as f64;

    Some((0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.)
}
