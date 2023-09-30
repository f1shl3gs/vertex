use bytes::Bytes;

use crate::aggregate::{Mode, Rule};

/// CRI log format: `TIMESTAMP STREAM TAG CONTENT`.
/// Tag `P` for partial
/// Tag `F` for full
///
/// Example:
/// ```text
/// 2016-10-06T00:17:09.669794202Z stdout P log content 1
/// 2016-10-06T00:17:09.669794203Z stdout P log content 2
/// 2016-10-06T00:17:09.669794203Z stdout F log content 3
/// ```
pub struct Cri;

impl Rule for Cri {
    fn is_start(&self, _line: &Bytes) -> bool {
        todo!()
    }

    fn is_condition(&self, _line: &Bytes) -> bool {
        todo!()
    }

    #[inline]
    fn mode(&self) -> Mode {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    #[allow(unused_variables)]
    fn merge() {
        let input = [
            "2019-05-07T18:57:50.904275087+00:00 stdout P 1a. some ",
            "2019-05-07T18:57:51.904275088+00:00 stdout P multiline ",
            "2019-05-07T18:57:52.904275089+00:00 stdout F log",
            "2019-05-07T18:57:50.904275087+00:00 stderr P 1b. some ",
            "2019-05-07T18:57:51.904275088+00:00 stderr P multiline ",
            "2019-05-07T18:57:52.904275089+00:00 stderr F log",
            "2019-05-07T18:57:53.904275090+00:00 stdout P 2a. another ",
            "2019-05-07T18:57:54.904275091+00:00 stdout P multiline ",
            "2019-05-07T18:57:55.904275092+00:00 stdout F log",
            "2019-05-07T18:57:53.904275090+00:00 stderr P 2b. another ",
            "2019-05-07T18:57:54.904275091+00:00 stderr P multiline ",
            "2019-05-07T18:57:55.904275092+00:00 stderr F log",
            "2019-05-07T18:57:56.904275093+00:00 stdout F 3a. non multiline 1",
            "2019-05-07T18:57:57.904275094+00:00 stdout F 4a. non multiline 2",
            "2019-05-07T18:57:56.904275093+00:00 stderr F 3b. non multiline 1",
            "2019-05-07T18:57:57.904275094+00:00 stderr F 4b. non multiline 2",
        ];

        let want = [
            "1a. some multiline log",
            "1b. some multiline log",
            "2a. another multiline log",
            "2b. another multiline log",
            "3a. non multiline 1",
            "4a. non multiline 2",
            "3b. non multiline 1",
            "4b. non multiline 2",
        ];
    }
}
