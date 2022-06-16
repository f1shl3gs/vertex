use event::{tags, Metric};

use crate::built_info;

pub fn built_info() -> Metric {
    let version = built_info::PKG_VERSION;
    let target = built_info::TARGET;
    let debug = built_info::DEBUG;

    Metric::gauge_with_tags(
        "build_info",
        "A metric with a constant '1' value labeled by version, revision, branch, and rust version from vertex was built",
        1,
        tags!(
            "version" => version,
            "target" => target,
            "debug" => debug,
            "rustc_version" => built_info::RUSTC_VERSION,
            "rustc_channel" => built_info::RUSTC_CHANNEL,
            "git" => format!("{}/{}", built_info::GIT_BRANCH, built_info::GIT_HASH),
        )
    )
}
