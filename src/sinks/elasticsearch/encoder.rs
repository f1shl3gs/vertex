#[derive(Debug, Clone, PartialEq)]
pub struct ElasticsearchEncoder {
    pub doc_type: String,
    pub suppress_type_name: bool,
    pub transformer: Transformer,
}
