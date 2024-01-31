use tracing::{
    field,
    span::{self, Attributes},
    subscriber::Interest,
    Event, Level, Metadata, Subscriber,
};
use tracing_subscriber::{
    filter::{Directive, Filtered},
    layer::{self, Context},
    registry::LookupSpan,
    Layer,
};

mod macros {

    #[macro_export]
    macro_rules! audit_span {
        () => {
            ::tracing::info_span!(
                target: $crate::serve::layer::audit::Audit::TARGET,
                $crate::serve::layer::audit::Audit::SPAN_ROOT_NAME)
        };
    }

    #[macro_export]
    macro_rules! audit {
        ($($arg:tt)*) => {
            ::tracing::event!(
                name: $crate::serve::layer::audit::Audit::EVENT_NAME,
                target: $crate::serve::layer::audit::Audit::TARGET,
                ::tracing::Level::TRACE, $($arg)*)
        };
    }
}

pub struct Audit;

impl Audit {
    pub const TARGET: &'static str = "__audit";
    pub const SPAN_ROOT_NAME: &'static str = "audit.root";
    pub const EVENT_NAME: &'static str = "audit.event";

    const EMIT_TARGET: &'static str = "audit";
    const EMIT_EVENT_NAME: &'static str = "audit";

    // follow opentelemetry semantic conventions
    pub const USER_ID: &'static str = "enduser.id";
    pub const OPERATION: &'static str = "operation";
    pub const RESULT: &'static str = "result";

    pub fn directive() -> Directive {
        let directive = format!("{emit}=info", emit = Self::EMIT_TARGET);

        directive.parse().expect("Invalid directive")
    }
}

/// Create AuditLayer
pub fn layer<S>() -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let layer = AuditLayer::new();
    let filter = AuditFilter::new();
    Filtered::new(layer, filter)
}

pub struct AuditFilter;

impl AuditFilter {
    fn new() -> Self {
        Self {}
    }

    fn is_enabled(&self, meta: &Metadata<'_>) -> bool {
        meta.target() == Audit::TARGET
    }
}

impl<S> layer::Filter<S> for AuditFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        self.is_enabled(meta)
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        if self.is_enabled(meta) {
            Interest::always()
        } else {
            Interest::never()
        }
    }
}

pub struct AuditLayer {}

impl AuditLayer {
    fn new() -> Self {
        Self {}
    }
}

struct AuditEventVisitor<'a> {
    ctx: &'a mut AuditContext,
}

impl<'a> field::Visit for AuditEventVisitor<'a> {
    fn record_debug(&mut self, _field: &field::Field, _value: &dyn std::fmt::Debug) {
        // do nothing
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        match field.name() {
            Audit::USER_ID => self.ctx.user_id = Some(value.to_owned()),
            Audit::OPERATION => self.ctx.operation = Some(value.to_string()),
            Audit::RESULT => self.ctx.result = Some(value.to_string()),
            _ => {}
        }
    }
}

struct AuditContext {
    user_id: Option<String>,
    operation: Option<String>,
    result: Option<String>,
}

impl AuditContext {
    fn new() -> Self {
        Self {
            user_id: None,
            operation: None,
            result: None,
        }
    }
}

impl<S> Layer<S> for AuditLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    /// If new span is audit root span, create AuditContext and insert to extensions.
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != Audit::SPAN_ROOT_NAME {
            return;
        }
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        extensions.insert(AuditContext::new());
    }

    /// If event is in audit span, visit event and store audit information to extensions
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if event.metadata().name() != Audit::EVENT_NAME {
            return;
        }

        // traverse span tree to find audit context
        let Some(span) = ctx.lookup_current() else {
            return;
        };
        let Some(audit_span) = span
            .scope()
            .from_root()
            .find(|span| span.metadata().name() == Audit::SPAN_ROOT_NAME)
        else {
            return;
        };
        let mut extension = audit_span.extensions_mut();
        let Some(audit_ctx) = extension.get_mut::<AuditContext>() else {
            return;
        };

        event.record(&mut AuditEventVisitor { ctx: audit_ctx });
    }

    /// If audit root span is closed, write a audit log
    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");
        if span.metadata().name() != Audit::SPAN_ROOT_NAME {
            return;
        }
        let mut extensions = span.extensions_mut();
        let Some(AuditContext {
            user_id,
            operation,
            result,
        }) = extensions.remove::<AuditContext>()
        else {
            return;
        };

        let user_id = user_id.as_deref().unwrap_or("?");
        let operation = operation.as_deref().unwrap_or("?");
        let result = result.as_deref().unwrap_or("?");

        tracing::event!(
            name: Audit::EMIT_EVENT_NAME,
            target: Audit::EMIT_TARGET,
            Level::INFO,
            { Audit::USER_ID } = user_id,
            { Audit::OPERATION } = operation,
            { Audit::RESULT } = result,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::{info, info_span};
    use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

    use super::*;

    struct TestLayer<F> {
        on_event: F,
    }

    impl<S, F> Layer<S> for TestLayer<F>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
        F: Fn(&Event<'_>) + 'static,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            (self.on_event)(event);
        }
    }

    // Mock audit layer usecase
    mod usecase_authorize_scenario {
        use tracing::info_span;

        use crate::{audit, audit_span, serve::layer::audit::Audit};

        pub fn root() {
            let span = audit_span!();
            let _enter = span.enter();
            usecase();
        }

        fn usecase() {
            let span = info_span!("usecase");
            let _enter = span.enter();
            authorize();
            ops();
            audit!(
                { Audit::OPERATION } = "create_foo",
                { Audit::RESULT } = "success",
            );
        }

        fn authorize() {
            let span = info_span!("authorize");
            let _enter = span.enter();
            audit!({ Audit::USER_ID } = "user-a",);
        }

        fn ops() {
            let span = info_span!("ops");
            let _enter = span.enter();
        }
    }

    #[test]
    fn usecase_authorize_scenario() {
        let ctx = Arc::new(Mutex::new(AuditContext::new()));
        let ctx2 = Arc::clone(&ctx);
        let on_event = move |event: &Event<'_>| {
            if event.metadata().name() == Audit::EMIT_EVENT_NAME {
                let mut ctx = ctx2.lock().unwrap();
                event.record(&mut AuditEventVisitor { ctx: &mut ctx });
            }
        };
        let test_layer = TestLayer { on_event };
        let subscriber = tracing_subscriber::registry()
            .with(layer())
            .with(test_layer);

        tracing::subscriber::with_default(subscriber, || {
            usecase_authorize_scenario::root();
        });

        let ctx = Arc::into_inner(ctx).unwrap().into_inner().unwrap();

        assert_eq!(ctx.user_id.as_deref(), Some("user-a"));
        assert_eq!(ctx.operation.as_deref(), Some("create_foo"));
        assert_eq!(ctx.result.as_deref(), Some("success"));
    }

    #[test]
    fn no_audit_span() {
        let on_event = |event: &Event<'_>| {
            if event.metadata().name() == Audit::EMIT_EVENT_NAME {
                panic!("should not called");
            }
        };
        let test_layer = TestLayer { on_event };
        let subscriber = tracing_subscriber::registry()
            .with(layer())
            .with(test_layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = info_span!("ignore");
            let _enter = span.enter();
            info!("ignore");
        });
    }

    #[test]
    fn emit_target_dose_not_equal_target() {
        assert!(Audit::TARGET != Audit::EMIT_TARGET);
    }
}
