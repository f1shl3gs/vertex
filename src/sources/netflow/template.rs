use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Field {
    pub typ: u16,
    pub length: u16,

    #[allow(dead_code)]
    pub pen: Option<u32>,
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

#[inline]
fn template_key(version: u16, odid: u32, template_id: u16) -> u64 {
    ((version as u64) << 48) | ((odid as u64) << 16) | template_id as u64
}

#[derive(Default)]
pub struct TemplateCache {
    inner: HashMap<u64, Template>,
}

impl TemplateCache {
    pub fn get(&self, version: u16, odid: u32, template_id: u16) -> Option<&Template> {
        let key = template_key(version, odid, template_id);
        self.inner.get(&key)
    }

    pub fn add(
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
