use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use configurable::Configurable;
use glob::{Pattern, PatternError, glob};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tail::{CheckpointsView, Fingerprint, Provider};
use tokio::time::Interval;

mod single_or_vec {
    use std::fmt::Formatter;

    use serde::de::{Error, SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};

    struct SingleOrVecVisitor;

    impl<'de> Visitor<'de> for SingleOrVecVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("a single string or an array of strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(vec![v.to_owned()])
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(vec![v])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut keys = Vec::new();
            while let Some(key) = seq.next_element::<String>()? {
                keys.push(key);
            }

            Ok(keys)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<String>, D::Error> {
        deserializer.deserialize_any(SingleOrVecVisitor)
    }

    pub fn serialize<S: Serializer>(keys: &Vec<String>, s: S) -> Result<S::Ok, S::Error> {
        if keys.len() == 1 {
            return s.serialize_str(&keys[0]);
        }

        let mut seq = s.serialize_seq(Some(keys.len()))?;
        for key in keys {
            seq.serialize_element(&key)?;
        }
        seq.end()
    }

    #[cfg(test)]
    mod tests {
        use serde::{Deserialize, Serialize};

        #[derive(Deserialize, Serialize)]
        struct Container {
            #[serde(with = "super")]
            keys: Vec<String>,
        }

        #[test]
        fn deserialize() {
            let single = r#"keys: foo"#;
            let container = serde_yaml::from_str::<Container>(single).unwrap();
            assert_eq!(container.keys, vec!["foo"]);

            let single_value = r#"key:\n  - foo"#;
            let container = serde_yaml::from_str::<Container>(single_value).unwrap();
            assert_eq!(container.keys, vec!["foo"]);
        }

        #[test]
        fn serialize() {
            let input = Container {
                keys: vec!["foo".to_string()],
            };
            let got = serde_yaml::to_string(&input).unwrap();
            assert_eq!(got, "keys: foo\n");

            let input = Container {
                keys: vec!["foo".to_string(), "bar".to_string()],
            };
            let got = serde_yaml::to_string(&input).unwrap();
            assert_eq!(got, "keys:\n- foo\n- bar\n");
        }
    }
}

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
pub enum Direction {
    #[default]
    Ascending,

    Descending,
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Sort {
    /// Keys to sort the grouped results
    #[serde(with = "single_or_vec")]
    by: Vec<String>,

    /// Direction of grouped results
    #[serde(default)]
    direction: Direction,
}

/// `Ordering` is very useful to prevent data loss when file rotated.
///
/// It groups the active and rotated files, and then sort them. With this setting,
/// vertex can understand the rotate behavior better, and reading rotated and
/// active files sequentially, rather than parallelly.
#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Ordering {
    /// Regular expression used for matching file path, then grouping and sorting.
    ///
    /// NOTE: Should contain at least one named capture which might be used in sort config.
    #[serde(with = "framework::config::serde_regex")]
    pattern: Regex,

    /// Named capture's value to build a unique key when grouping
    #[serde(with = "single_or_vec")]
    group_by: Vec<String>,

    /// The number of files to track
    #[serde(default, skip_serializing_if = "Option::is_none")]
    limit: Option<NonZeroUsize>,

    /// Sort of the grouped paths, and make sure the latest one is the first
    sort: Sort,
}

/*
/// Custom deserialize & serialize is much complexer than `derive`, but it
/// can raise errors when deserializing
impl<'de> Deserialize<'de> for Ordering {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SingleOrVecVisitor;

        impl<'de> Visitor<'de> for SingleOrVecVisitor {
            type Value = Vec<String>;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a single string or an array of strings")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                todo!()
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                todo!()
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                todo!()
            }
        }

        struct OrderingVisitor;

        impl<'de> Visitor<'de> for OrderingVisitor {
            type Value = Ordering;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("an ordering struct")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut pattern = None;
                let mut group_by = None;
                let mut limit = None;
                let mut sort = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "pattern" => {
                            if pattern.is_some() {
                                return Err(serde::de::Error::duplicate_field("pattern"));
                            }

                            let value = map.next_value::<&str>()?;

                            match Regex::new(value) {
                                Ok(re) => {
                                    if re.capture_names().any(|name| name.is_some()) {
                                        pattern = Some(re);
                                        continue;
                                    }

                                    return Err(serde::de::Error::invalid_value(
                                        Unexpected::Str(value),
                                        &"A valid regex pattern with at least one named capture",
                                    ));
                                }
                                Err(err) => {
                                    return Err(serde::de::Error::invalid_value(
                                        Unexpected::Str(value),
                                        &(err.to_string().as_str()),
                                    ));
                                }
                            }
                        }
                        "group_by" => {
                            if group_by.is_some() {
                                return Err(serde::de::Error::duplicate_field("group_by"));
                            }

                            if let Ok(array) = map.next_value::<Vec<String>>() {
                                group_by = Some(array);
                                continue;
                            }

                            if let Ok(value) = map.next_value::<String>() {
                                group_by = Some(vec![value]);
                                continue;
                            }

                            return Err(serde::de::Error::invalid_type(
                                Unexpected::Other("aaaa"),
                                &"string or an array of string",
                            ))
                        }
                        "limit" => {
                            if limit.is_some() {
                                return Err(serde::de::Error::duplicate_field("limit"));
                            }

                            let value = map.next_value::<NonZeroUsize>()?;
                            limit = Some(value);
                        }
                        "sort" => {
                            if sort.is_some() {
                                return Err(serde::de::Error::duplicate_field("sort"));
                            }

                            let value = map.next_value::<Sort>()?;
                            sort = Some(value);
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                key,
                                &["pattern", "group_by", "limit", "sort"],
                            ));
                        }
                    }
                }

                let Some(pattern) = pattern else {
                    return Err(serde::de::Error::missing_field("pattern"));
                };

                let Some(group_by) = group_by else {
                    return Err(serde::de::Error::missing_field("group_by"));
                };

                let Some(sort) = sort else {
                    return Err(serde::de::Error::missing_field("sort"));
                };

                // validate

                Ok(Ordering {
                    pattern,
                    group_by,
                    limit,
                    sort
                })
            }
        }

        deserializer.deserialize_any(OrderingVisitor)
    }
}
*/

impl Ordering {
    pub fn validate(&self) -> Result<(), String> {
        let names = self.pattern.capture_names().flatten().collect::<Vec<_>>();

        if names.is_empty() {
            return Err("pattern must contain at least one named capture".to_string());
        }

        for name in &self.group_by {
            if !names.contains(&name.as_str()) {
                return Err(format!(
                    "`{name}` in `group_by` is not present in `pattern`"
                ));
            }
        }

        for name in &self.sort.by {
            if !names.contains(&name.as_str()) {
                return Err(format!("`{name}` in `sort.by` is not present in `pattern`"));
            }
        }

        Ok(())
    }

    fn group(
        &self,
        paths: impl Iterator<Item = PathBuf>,
    ) -> HashMap<String, Vec<(PathBuf, BTreeMap<String, String>)>> {
        let mut map: HashMap<String, Vec<(PathBuf, BTreeMap<String, String>)>> = HashMap::new();

        for path in paths {
            let path_str = path.to_string_lossy();
            let Some(captures) = self.pattern.captures(path_str.as_ref()) else {
                trace!(
                    message = "path cannot match the `group_by` regex",
                    group_by = self.pattern.as_str(),
                    ?path,
                );

                continue;
            };

            let captured = self
                .pattern
                .capture_names()
                .flatten()
                .filter_map(|name| {
                    let value = captures.name(name)?;
                    Some((name.to_string(), value.as_str().to_string()))
                })
                .collect::<BTreeMap<String, String>>();

            let mut key = String::new();
            for (index, name) in self.group_by.iter().enumerate() {
                match captures.name(name) {
                    Some(m) => {
                        if index != 0 {
                            key.push(',');
                        }
                        key.push_str(m.as_str());
                    }
                    None => unreachable!(),
                }
            }

            match map.entry(key) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().push((path, captured));
                }
                Entry::Vacant(entry) => {
                    entry.insert(vec![(path, captured)]);
                }
            };
        }

        map.values_mut().for_each(|paths| {
            paths.sort_by(|(_, a), (_, b)| {
                let am = self
                    .sort
                    .by
                    .iter()
                    .filter_map(|key| a.get(key))
                    .collect::<Vec<_>>();

                let bm = self
                    .sort
                    .by
                    .iter()
                    .filter_map(|key| b.get(key))
                    .collect::<Vec<_>>();

                match self.sort.direction {
                    Direction::Ascending => am.cmp(&bm),
                    Direction::Descending => bm.cmp(&am),
                }
            });

            if let Some(limit) = self.limit {
                paths.truncate(limit.get());
            }
        });

        map
    }
}

pub struct GlobProvider {
    include: Vec<String>,
    exclude: Vec<Pattern>,
    ordering: Option<Ordering>,
    // if ordering is specified, `ignore_older_than` only works for the latest item
    ignore_older_than: Option<Duration>,
    checkpoints: CheckpointsView,

    ticker: Interval,
}

impl GlobProvider {
    pub fn new(
        include: Vec<String>,
        exclude: &[String],
        interval: Duration,
        ordering: Option<Ordering>,
        ignore_older_than: Option<Duration>,
        checkpoints: CheckpointsView,
    ) -> Result<Self, PatternError> {
        let exclude = exclude
            .iter()
            .map(|item| Pattern::new(item.as_str()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            include,
            exclude,
            ordering,
            ignore_older_than,
            checkpoints,
            ticker: tokio::time::interval(interval),
        })
    }
}

impl Provider for GlobProvider {
    type Metadata = BTreeMap<String, String>;

    async fn scan(&mut self) -> std::io::Result<Vec<(PathBuf, Self::Metadata)>> {
        self.ticker.tick().await;

        let mut paths = vec![];
        for pattern in &self.include {
            match glob(pattern) {
                Ok(matches) => matches.into_iter().flatten().for_each(|item| {
                    if paths.contains(&item) {
                        return;
                    }

                    if self
                        .exclude
                        .iter()
                        .any(|pattern| pattern.matches_path(&item))
                    {
                        return;
                    }

                    paths.push(item);
                }),
                Err(err) => {
                    warn!(message = "include glob pattern match failed", pattern, ?err);

                    continue;
                }
            }
        }

        let now = SystemTime::now();
        let paths = match &self.ordering {
            Some(ordering) => {
                ordering
                    .group(paths.into_iter())
                    .into_values()
                    .filter_map(|grouped: Vec<(PathBuf, BTreeMap<String, String>)>| {
                        #[cfg(test)]
                        {
                            for (path, _meta) in &grouped {
                                println!("{path:?}");
                            }

                            println!()
                        }

                        // There are 3 situations we might meet
                        //
                        // 1. there are no checkpoints, then return the oldest one
                        // 2. the non-latest one have checkpoint
                        //    a. the size is match, return the newer one
                        //    b. the size is not match, return current one
                        // 3. the latest one have checkpoint, then return it unless it is
                        //    too old, no matter the checkpoint size

                        let mut last = None;
                        for (index, (path, meta)) in grouped.into_iter().enumerate() {
                            let Ok(stat) = path.metadata() else { continue };

                            let fingerprint = Fingerprint::from(&stat);
                            match self.checkpoints.get(&fingerprint) {
                                Some(offset) => {
                                    let size = offset.load(std::sync::atomic::Ordering::Acquire);

                                    if index == 0 {
                                        // always return the first and latest file with checkpoints,
                                        // unless it's too old

                                        // there are some data to catch up
                                        if stat.size() != size {
                                            return Some((path, meta));
                                        }

                                        // file is already catch up, then ignore it if it is too old
                                        if let Some(ignore_older_than) = self.ignore_older_than {
                                            let Ok(modified) = stat.modified() else {
                                                continue;
                                            };

                                            if now - ignore_older_than > modified {
                                                // latest file is too old, ignore it
                                                return None;
                                            }
                                        }

                                        return Some((path, meta));
                                    }

                                    if size < stat.size() {
                                        // there are some data to catch up
                                        return Some((path, meta));
                                    } else if size == stat.size() {
                                        // no data need to catch up, return the previous one
                                        return last;
                                    }
                                }
                                None => {
                                    // try the previous file
                                    last = Some((path, meta));
                                }
                            }
                        }

                        last
                    })
                    .collect::<Vec<_>>()
            }
            None => paths
                .into_iter()
                .filter(|path| match self.ignore_older_than {
                    Some(ignore_older_than) => {
                        let Ok(metadata) = path.metadata() else {
                            return false;
                        };

                        let Ok(modified) = metadata.modified() else {
                            return false;
                        };

                        now - ignore_older_than <= modified
                    }
                    None => true,
                })
                .map(|path| (path, BTreeMap::new()))
                .collect::<Vec<_>>(),
        };

        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::io::Write;
    use std::num::NonZeroUsize;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use regex::Regex;
    use tail::{Checkpointer, Fingerprint, Provider};
    use testify::temp_dir;

    use super::{Direction, GlobProvider, Ordering, Sort};

    #[test]
    fn deserialize() {
        let input = r#"
pattern: /path/(?<app>\S+)/(?<stage>\S+)/(?<seq>\S+).log
group_by: app
sort:
  by: seq
"#;
        let _ordering = serde_yaml::from_str::<Ordering>(input).unwrap();
    }

    #[tokio::test]
    async fn just_include() {
        let root = temp_dir();

        std::fs::File::create(root.join("a.log")).unwrap();
        std::fs::File::create(root.join("b.log")).unwrap();

        let checkpointer = Checkpointer::load(root.clone()).unwrap();

        let mut provider = GlobProvider::new(
            vec![format!("{}/*.log", root.to_string_lossy())],
            &[],
            Duration::from_millis(100),
            None,
            None,
            checkpointer.view(),
        )
        .unwrap();

        let matches = provider.scan().await.unwrap();
        assert_eq!(
            matches,
            vec![
                (root.join("a.log"), Default::default()),
                (root.join("b.log"), Default::default()),
            ]
        )
    }

    #[tokio::test]
    async fn with_exclude() {
        let root = temp_dir();

        std::fs::File::create(root.join("a.log")).unwrap();
        std::fs::File::create(root.join("b.log")).unwrap();

        let checkpointer = Checkpointer::load(root.clone()).unwrap();

        let mut provider = GlobProvider::new(
            vec![format!("{}/*.log", root.to_string_lossy())],
            &[format!("{}/a.log", root.to_string_lossy())],
            Duration::from_millis(100),
            None,
            None,
            checkpointer.view(),
        )
        .unwrap();

        let matches = provider.scan().await.unwrap();
        assert_eq!(matches, vec![(root.join("b.log"), Default::default())]);
    }

    #[tokio::test]
    async fn ignore_older_than() {
        let root = temp_dir();

        std::fs::File::create(root.join("a.log")).unwrap();
        let file = std::fs::File::create(root.join("b.log")).unwrap();
        file.set_modified(
            SystemTime::now()
                .checked_sub(Duration::from_secs(20))
                .unwrap(),
        )
        .unwrap();

        let checkpointer = Checkpointer::load(root.clone()).unwrap();

        let mut provider = GlobProvider::new(
            vec![format!("{}/*.log", root.to_string_lossy())],
            &[],
            Duration::from_millis(100),
            None,
            Some(Duration::from_secs(10)),
            checkpointer.view(),
        )
        .unwrap();

        let matches = provider.scan().await.unwrap();
        assert_eq!(matches, vec![(root.join("a.log"), Default::default())])
    }

    fn prepare(root: &Path) -> Vec<PathBuf> {
        let paths = vec![
            root.join("foo/1.log"),
            root.join("foo/2.log"),
            root.join("foo/3.log"),
            root.join("bar/4.log"),
            root.join("bar/5.log"),
        ];

        std::fs::create_dir_all(root.join("foo")).unwrap();
        std::fs::create_dir_all(root.join("bar")).unwrap();

        for path in &paths {
            let mut file = std::fs::File::create_new(path).unwrap();
            file.write_all("hello".as_bytes()).unwrap();
        }

        paths
    }

    #[test]
    fn ordering() {
        let root = temp_dir();

        let paths = prepare(&root);

        let mut config = Ordering {
            pattern: Regex::new(&format!(
                r#"{}/(?<app>\S+)/(?<seq>\S+).log"#,
                root.to_string_lossy()
            ))
            .unwrap(),
            group_by: vec!["app".to_string()],
            limit: None,
            sort: Sort {
                by: vec!["seq".to_string()],
                direction: Direction::Ascending,
            },
        };

        config.validate().unwrap();

        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[1, 2, 3]), ("bar", &[4, 5])]);

        config.sort.direction = Direction::Descending;
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[3, 2, 1]), ("bar", &[5, 4])]);

        // with limit
        config.sort.direction = Direction::Ascending;
        config.limit = Some(NonZeroUsize::new(1).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[1]), ("bar", &[4])]);

        config.sort.direction = Direction::Descending;
        config.limit = Some(NonZeroUsize::new(1).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[3]), ("bar", &[5])]);

        config.sort.direction = Direction::Ascending;
        config.limit = Some(NonZeroUsize::new(2).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[1, 2]), ("bar", &[4, 5])]);

        config.sort.direction = Direction::Descending;
        config.limit = Some(NonZeroUsize::new(2).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[3, 2]), ("bar", &[5, 4])]);

        config.sort.direction = Direction::Ascending;
        config.limit = Some(NonZeroUsize::new(3).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[1, 2, 3]), ("bar", &[4, 5])]);

        config.sort.direction = Direction::Descending;
        config.limit = Some(NonZeroUsize::new(3).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[3, 2, 1]), ("bar", &[5, 4])]);

        config.sort.direction = Direction::Ascending;
        config.limit = Some(NonZeroUsize::new(4).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[1, 2, 3]), ("bar", &[4, 5])]);

        config.sort.direction = Direction::Descending;
        config.limit = Some(NonZeroUsize::new(4).unwrap());
        let grouped = config.group(paths.clone().into_iter());
        assert_grouped(&root, grouped, &[("foo", &[3, 2, 1]), ("bar", &[5, 4])]);
    }

    #[tokio::test]
    async fn group_forward() {
        let root = temp_dir();

        let paths = prepare(&root);
        let f1 = &paths[0];
        let f2 = &paths[1];
        let f3 = &paths[2];
        // let b4 = &paths[3];
        // let b5 = &paths[4];

        let ordering = Ordering {
            pattern: Regex::new(&format!(
                r#"{}/(?<app>\S+)/(?<seq>\S+).log"#,
                root.to_string_lossy()
            ))
            .unwrap(),
            group_by: vec!["app".to_string()],
            limit: None,
            sort: Sort {
                by: vec!["seq".to_string()],
                direction: Direction::Descending,
            },
        };

        ordering.validate().unwrap();

        let checkpointer = Checkpointer::load(root.clone()).unwrap();

        let mut provider = GlobProvider::new(
            vec![format!("{}/*/*.log", root.to_string_lossy())],
            &[],
            Duration::from_secs(1),
            Some(ordering),
            Some(Duration::from_secs(60)), // 1min
            checkpointer.view(),
        )
        .unwrap();

        // there are no checkpoints, return the oldest one
        let result = provider.scan().await.unwrap();
        assert_scanned(&root, result, &[("bar", 4), ("foo", 1)]); // the smaller, the older
        println!("------");

        // foo 1, because we need to catch up
        let offset = checkpointer.insert(Fingerprint::from(&f1.metadata().unwrap()), 0);
        let result = provider.scan().await.unwrap();
        assert_scanned(&root, result, &[("bar", 4), ("foo", 1)]);
        println!("------");

        // foo 1, because we need to catch up
        offset.fetch_add(4, std::sync::atomic::Ordering::Relaxed);
        let result = provider.scan().await.unwrap();
        assert_scanned(&root, result, &[("bar", 4), ("foo", 1)]);
        println!("------");

        // foo 1 is done, foo 2 is what we want
        offset.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let result = provider.scan().await.unwrap();
        assert_scanned(&root, result, &[("bar", 4), ("foo", 2)]);
        println!("------");

        // set f2 done, so we got f3
        let _offset = checkpointer.insert(Fingerprint::from(&f2.metadata().unwrap()), 5);
        let result = provider.scan().await.unwrap();
        assert_scanned(&root, result, &[("bar", 4), ("foo", 3)]);
        println!("------");

        // set f3 done, we still got f3 because it is the latest one
        let _offset = checkpointer.insert(Fingerprint::from(&f3.metadata().unwrap()), 5);
        let result = provider.scan().await.unwrap();
        assert_scanned(&root, result, &[("bar", 4), ("foo", 3)]);
        println!("------");

        // mark f3 too old, so ignore it
        let file = std::fs::File::open(f3).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(120))
            .unwrap();
        let result = provider.scan().await.unwrap();
        assert_scanned(&root, result, &[("bar", 4)]);
        println!("------");
    }

    fn assert_scanned(
        root: &Path,
        mut got: Vec<(PathBuf, BTreeMap<String, String>)>,
        want: &[(&str, u32)],
    ) {
        got.sort_by(|(a, _), (b, _)| a.cmp(b));

        assert_eq!(got.len(), want.len());

        for ((path, meta), (app, seq)) in got.iter().zip(want.iter()) {
            assert_eq!(
                *path,
                root.join(format!("{app}/{seq}.log")),
                "got: {path:?}, want: {app}/{seq}.log",
            );

            assert_eq!(meta.len(), 2);
            assert_eq!(meta.get("app").unwrap(), app);
            assert_eq!(meta.get("seq").unwrap(), &seq.to_string());
        }
    }

    fn assert_grouped(
        root: &Path,
        got: HashMap<String, Vec<(PathBuf, BTreeMap<String, String>)>>,
        want: &[(&str, &[i32])],
    ) {
        let want = want
            .iter()
            .map(|(app, seqs)| {
                let paths = seqs
                    .iter()
                    .map(|seq| {
                        let path = root.join(format!("{app}/{seq}.log"));

                        let mut meta = BTreeMap::new();
                        meta.insert("app".to_string(), app.to_string());
                        meta.insert("seq".to_string(), seq.to_string());

                        (path, meta)
                    })
                    .collect::<Vec<_>>();

                (app.to_string(), paths)
            })
            .collect::<HashMap<_, _>>();

        assert_eq!(got, want)
    }
}
