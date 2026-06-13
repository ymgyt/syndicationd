use crate::{event::Event, operation::Operation};

use super::{
    DriverContext, auth::AuthDriver, feed::FeedDriver, feed_events::FeedEventDriver,
    github::GitHubDriver, interaction::InteractionDriver,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct OperationDispatcher {
    _marker: (),
}

impl OperationDispatcher {
    pub(super) fn new() -> Self {
        Self { _marker: () }
    }

    pub(super) fn dispatch(self, operation: Operation, cx: &mut DriverContext<'_>) -> Vec<Event> {
        let Self { _marker: () } = self;

        match operation {
            Operation::StartDeviceFlow { provider } => AuthDriver::start_device_flow(cx, provider),
            Operation::PollDeviceFlowAccessToken {
                provider,
                device_authorization,
            } => AuthDriver::poll_device_flow_access_token(cx, provider, *device_authorization),
            Operation::OpenFeedSubscriptionEditor => {
                InteractionDriver::open_feed_subscription_editor(cx)
            }
            Operation::OpenFeedEditionEditor { prompt } => {
                InteractionDriver::open_feed_edition_editor(cx, prompt.as_str())
            }
            Operation::SubscribeFeed { input } => FeedDriver::subscribe_feed(cx, input),
            Operation::RefreshFeed { url } => FeedDriver::refresh_feed(cx, url),
            Operation::FetchFeedRefreshStatus {
                url,
                request_id,
                remaining,
            } => FeedDriver::fetch_feed_refresh_status(cx, url, request_id, remaining),
            Operation::ScheduleFeedRefreshPoll {
                url,
                request_id,
                remaining,
            } => FeedDriver::schedule_feed_refresh_poll(cx, url, request_id, remaining),
            Operation::FetchSubscription {
                populate,
                after,
                first,
            } => FeedDriver::fetch_subscription(cx, populate, after, first),
            Operation::FetchEntries {
                populate,
                after,
                first,
            } => FeedDriver::fetch_entries(cx, populate, after, first, false),
            Operation::FetchInitialFeedView {
                subscriptions_first,
                timeline_first,
            } => FeedDriver::fetch_initial_feed_view(cx, subscriptions_first, timeline_first),
            Operation::RefetchTimelineEntries {
                populate,
                after,
                first,
            } => FeedDriver::fetch_entries(cx, populate, after, first, true),
            Operation::StartFeedEventSubscription => FeedEventDriver::start_subscription(cx),
            Operation::ScheduleFeedViewReload {
                feeds_first,
                entries_first,
            } => FeedDriver::schedule_feed_view_reload(cx, feeds_first, entries_first),
            Operation::UnsubscribeFeed { url } => FeedDriver::unsubscribe_feed(cx, url),
            Operation::ScheduleFeedViewSync => FeedDriver::schedule_feed_view_sync(cx),
            Operation::ScheduleTimelineReload => FeedDriver::schedule_timeline_reload(cx),
            Operation::FetchGitHubNotifications { populate, params } => {
                GitHubDriver::fetch_notifications(cx, populate, params)
            }
            Operation::FetchGitHubNotificationDetails { contexts } => {
                GitHubDriver::fetch_notification_details(cx, contexts)
            }
            Operation::MarkGitHubNotificationAsDone { id } => {
                GitHubDriver::mark_notification_as_done(cx, id)
            }
            Operation::UnsubscribeGitHubThread { id } => GitHubDriver::unsubscribe_thread(cx, id),
            Operation::OpenBrowser { url } => InteractionDriver::open_browser(cx, url),
            Operation::OpenTextBrowser { url } => InteractionDriver::open_text_browser(cx, url),
            Operation::ForceRedrawTerminal => InteractionDriver::force_redraw_terminal(cx),
        }
    }
}
