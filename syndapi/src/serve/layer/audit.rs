use std::io::Write;

use tracing::{
    field,
    span::{self, Attributes},
    Event, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// let subscriber = tracing_subscriber::registry()
///    .with(layer(std::io::stdout);
pub fn layer<W: MakeWriter>(make_writer: W) -> AuditLayer<W> {
    AuditLayer { make_writer }
}

pub trait MakeWriter {
    type Writer: Write;

    fn make_writer(&self) -> Self::Writer;
}

impl<F, W> MakeWriter for F
where
    F: Fn() -> W,
    W: std::io::Write,
{
    type Writer = W;

    fn make_writer(&self) -> Self::Writer {
        (self)()
    }
}

pub struct AuditLayer<W> {
    make_writer: W,
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
            "organization_id" => self.ctx.organization_id = Some(value.to_owned()),
            "operation" => self.ctx.operation = Some(value.to_owned()),
            "result" => self.ctx.result = Some(value.to_owned()),
            _ => {}
        }
    }
}

struct AuditContext {
    organization_id: Option<String>,
    operation: Option<String>,
    result: Option<String>,
}

impl AuditContext {
    fn new() -> Self {
        Self {
            organization_id: None,
            operation: None,
            result: None,
        }
    }
}

impl<S, W> Layer<S> for AuditLayer<W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: MakeWriter + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != "audit.root" {
            return;
        }
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        extensions.insert(AuditContext::new());
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if event.metadata().name() != "audit.event" {
            return;
        }

        // traverse span tree to find audit context
        let Some(span) = ctx.lookup_current() else {
            return;
        };
        let Some(audit_span) = span
            .scope()
            .from_root()
            .find(|span| span.metadata().name() == "audit.root")
        else {
            return;
        };
        let mut extension = audit_span.extensions_mut();
        let Some(audit_ctx) = extension.get_mut::<AuditContext>() else {
            return;
        };

        event.record(&mut AuditEventVisitor { ctx: audit_ctx });
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");
        if span.metadata().name() != "audit.root" {
            return;
        }
        let mut extensions = span.extensions_mut();
        let Some(AuditContext {
            organization_id,
            operation,
            result,
        }) = extensions.remove::<AuditContext>()
        else {
            return;
        };

        let mut writer = self.make_writer.make_writer();

        writeln!(
            writer,
            "organization_id: {organization_id:?}, operation: {operation:?}, result:{result:?}"
        )
        .ok(); // or panic
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

    use super::*;

    #[derive(Clone)]
    struct TestWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl TestWriter {
        fn new() -> Self {
            Self {
                buf: Arc::new(Mutex::new(Vec::new())),
            }
        }
        fn buf(self) -> Vec<u8> {
            Arc::into_inner(self.buf).unwrap().into_inner().unwrap()
        }
    }

    impl MakeWriter for TestWriter {
        type Writer = Self;

        fn make_writer(&self) -> Self::Writer {
            self.clone()
        }
    }

    impl std::io::Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let len = buf.len();
            self.buf.lock().unwrap().extend(buf);
            Ok(len)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    use tracing::{info, info_span};

    fn root() {
        let span = info_span!("audit.root");
        let _enter = span.enter();
        usecase();
        info!(name: "audit.event", result = "Success");
    }

    fn usecase() {
        let span = info_span!("usecase");
        let _enter = span.enter();
        authorize();
        ops();
    }

    fn authorize() {
        let span = info_span!("authorize");
        let _enter = span.enter();
        info!(name: "audit.event", organization_id = "org-a",);
    }

    fn ops() {
        let span = info_span!("ops");
        let _enter = span.enter();
        info!(name: "audit.event", operation = "CreateXxx");
    }

    #[test]
    fn handson() {
        let buf = TestWriter::new();
        let subscriber = tracing_subscriber::registry().with(layer(buf.clone()));

        tracing::subscriber::with_default(subscriber, || {
            root();
        });

        let buf = buf.buf();
        let buf = String::from_utf8_lossy(&buf);
        println!("Result: `{buf}`");
    }

    #[test]
    fn stdout_make_writer() {
        layer(std::io::stdout);
    }
}
