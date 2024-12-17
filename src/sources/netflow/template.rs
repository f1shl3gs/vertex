use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Field {
    pub typ: u16,
    pub length: u16,
    #[allow(dead_code)]
    pub pen: Option<u32>,
}

/// TemplateRecord is a single template that describes structure of a Flow Record
/// (the actual NetFlow data)
#[derive(Debug)]
pub struct TemplateRecord {
    /// Each of the newly generated template record is given a unique template id.
    /// This uniqueness is local to the observation domain that generated the
    /// template id. Template ids of data flow_sets are numbered from 256 to 65535.
    pub id: u16,

    // /// Number of fields in this template record. Because a template flow set
    // /// usually contains multiple template records, this field allows the `Collector`
    // /// to determine the end of the current template record and the start of the next.
    // pub field_count: u16,
    /// List of fields in this template records.
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub enum Template {
    Basic {
        fields: Vec<Field>,
    },
    Options {
        scopes: Vec<Field>,
        options: Vec<Field>,
    },
}

pub trait TemplateSystem {
    fn get(&self, version: u16, odid: u32, template_id: u16) -> Option<&Template>;

    fn add(
        &mut self,
        version: u16,
        odid: u32,
        template_id: u16,
        template: Template,
    ) -> Option<Template>;
}

#[inline]
fn template_key(version: u16, odid: u32, template_id: u16) -> u64 {
    ((version as u64) << 48) | ((odid as u64) << 16) | template_id as u64
}

#[derive(Debug, Default)]
pub struct BasicTemplateSystem {
    inner: HashMap<u64, Template>,
}

impl TemplateSystem for BasicTemplateSystem {
    fn get(&self, version: u16, odid: u32, template_id: u16) -> Option<&Template> {
        let key = template_key(version, odid, template_id);
        self.inner.get(&key)
    }

    fn add(
        &mut self,
        version: u16,
        odid: u32,
        template_id: u16,
        template: Template,
    ) -> Option<Template> {
        let key = template_key(version, odid, template_id);
        self.inner.insert(key, template)
    }
}
