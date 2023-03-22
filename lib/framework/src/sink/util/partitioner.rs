use event::Event;

use crate::partition::Partitioner;
use crate::template::Template;

#[derive(Clone)]
pub struct KeyPartitioner(Option<Template>);

impl KeyPartitioner {
    pub const fn new(template: Option<Template>) -> Self {
        Self(template)
    }
}

impl Partitioner for KeyPartitioner {
    type Item = Event;
    type Key = Option<String>;

    fn partition(&self, item: &Self::Item) -> Self::Key {
        self.0.as_ref().and_then(|tmpl| {
            tmpl.render_string(item)
                .map_err(|err| {
                    error!(
                        message = "Failed to render template",
                        ?err,
                        field = "tenant_id",
                        drop_event = false
                    );
                    // TODO: metrics
                    //
                    // emit!(&TemplateRenderingFailed {
                    //     err,
                    //     field: Some("tenant_id"),
                    //     drop_event: false,
                    // })
                })
                .ok()
        })
    }
}
