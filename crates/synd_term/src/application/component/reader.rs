use std::{cell::RefCell, sync::Arc};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Paragraph, Widget as _, Wrap},
};
use url::Url;

use super::{Component, RenderContext};
use crate::{
    application::{Generation, RequestError, RequestKind, effects::Effects},
    command::{Command, ReaderCommand},
    event::{Event, EventKind},
    keymap::Layer,
    operation::Operation,
    reader::{Article, ArticleBody},
};

#[derive(Default)]
pub(super) struct ReaderComponent {
    generation: Generation,
    state: State,
}

#[derive(Default)]
enum State {
    #[default]
    Closed,
    Loading(Url),
    Ready {
        article: Arc<Article>,
        body: Box<RefCell<ArticleBody>>,
    },
    Failed(String),
}

impl ReaderComponent {
    pub(super) fn apply_command(&mut self, command: ReaderCommand) -> Effects {
        match command {
            ReaderCommand::Open(url) => {
                self.generation.advance();
                self.state = State::Loading(url.clone());
                Effects::operations(Operation::FetchArticle {
                    generation: self.generation,
                    url,
                })
                .with_command(Command::Open)
            }
            ReaderCommand::Close => {
                self.state = State::Closed;
                Command::Close.into()
            }
            ReaderCommand::Scroll(by) => {
                if let State::Ready { body, .. } = &mut self.state {
                    body.get_mut().scroll(by);
                }
                Effects::NONE
            }
        }
    }
}

impl Component for ReaderComponent {
    fn subscriptions(&self) -> &'static [EventKind] {
        &[EventKind::ArticleFetched, EventKind::RequestCompleted]
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        if matches!(self.state, State::Loading(_)) {
            match event {
                Event::ArticleFetched {
                    generation,
                    article,
                    ..
                } if *generation == self.generation => {
                    self.state = State::Ready {
                        article: Arc::clone(article),
                        body: Box::default(),
                    };
                }
                Event::RequestCompleted {
                    kind: RequestKind::FetchArticle { generation },
                    result: Err(RequestError::Article(error)),
                    ..
                } if *generation == self.generation => {
                    self.state = State::Failed(format!("{error:#}"));
                }
                _ => {}
            }
        }
        Effects::NONE
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::Reader]
    }

    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        let area = Rect {
            height: cx.frame.status.y.saturating_sub(cx.frame.area.y),
            ..cx.frame.area
        };
        let [title_area, body_area] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)])
            .horizontal_margin(1)
            .areas(area);
        let title = match &self.state {
            State::Ready { article, .. } => article.title.as_str(),
            _ => "Reader",
        };
        Line::styled(title, cx.ui.theme.application_title).render(title_area, buf);
        match &self.state {
            State::Ready { article, body } => body.borrow_mut().render(article, body_area, buf),
            State::Loading(url) => Paragraph::new(format!("Loading {url}…"))
                .wrap(Wrap { trim: false })
                .render(body_area, buf),
            State::Failed(error) => Paragraph::new(error.as_str())
                .style(cx.ui.theme.error.message)
                .wrap(Wrap { trim: false })
                .render(body_area, buf),
            State::Closed => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::RequestId;

    #[test]
    fn a_late_result_cannot_replace_the_reopened_article() {
        let mut reader = ReaderComponent::default();
        let url = Url::parse("https://example.com/article").unwrap();
        let article =
            crate::reader::extract("<article><p>Old article body.</p></article>", &url).unwrap();
        let _ = reader.apply_command(ReaderCommand::Open(url.clone()));
        let first = reader.generation;
        let _ = reader.apply_command(ReaderCommand::Close);
        let _ = reader.apply_command(ReaderCommand::Open(url));
        let request_id = RequestId::new(1);
        let _ = reader.apply_event(&Event::ArticleFetched {
            request_id,
            generation: first,
            article: Arc::new(article),
        });
        let _ = reader.apply_event(&Event::RequestCompleted {
            request_id,
            kind: RequestKind::FetchArticle { generation: first },
            result: Err(RequestError::Article(anyhow::anyhow!("old failure"))),
        });
        assert!(matches!(reader.state, State::Loading(_)));

        let _ = reader.apply_event(&Event::RequestCompleted {
            request_id: RequestId::new(2),
            kind: RequestKind::FetchArticle {
                generation: reader.generation,
            },
            result: Err(RequestError::Article(anyhow::anyhow!("publisher offline"))),
        });
        assert!(
            matches!(reader.state, State::Failed(ref message) if message.contains("publisher offline"))
        );
    }
}
