use framework::sink::util::Compression;

#[derive(Debug)]
pub struct ElasticsearchRequestBuilder {
    pub compression: Compression,
    pub encoder: ElasticsearchEncoder,
}
