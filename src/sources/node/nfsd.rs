use std::convert::{TryFrom, TryInto};
use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::Error;
use super::nfs::{Network, V2Stats, V3Stats};

// ReplyCache models the "rc" line.
#[derive(Debug, Default, PartialEq)]
struct ReplyCache {
    hits: u64,
    misses: u64,
    no_cache: u64,
}

impl TryFrom<Vec<u64>> for ReplyCache {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 3 {
            return Err(format!("invalid ReplyCache line {v:?}").into());
        }

        Ok(Self {
            hits: v[0],
            misses: v[1],
            no_cache: v[2],
        })
    }
}

// FileHandles models the "fh" line.
#[derive(Debug, Default, PartialEq)]
struct FileHandles {
    stale: u64,
    total_lookups: u64,
    anon_lookups: u64,
    dir_no_cache: u64,
    no_dir_no_cache: u64,
}

impl TryFrom<Vec<u64>> for FileHandles {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 5 {
            return Err(format!("invalid FileHandles line {v:?}").into());
        }

        Ok(Self {
            stale: v[0],
            total_lookups: v[1],
            anon_lookups: v[2],
            dir_no_cache: v[3],
            no_dir_no_cache: v[4],
        })
    }
}

// InputOutput models the "io" line.
#[derive(Debug, Default, PartialEq)]
struct InputOutput {
    read: u64,
    write: u64,
}

impl TryFrom<Vec<u64>> for InputOutput {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 2 {
            return Err(format!("invalid InputOutput line {v:?}").into());
        }

        Ok(Self {
            read: v[0],
            write: v[1],
        })
    }
}

// Threads models the "th" line.
#[derive(Debug, Default, PartialEq)]
struct Threads {
    threads: u64,
    full_cnt: u64,
}

impl TryFrom<Vec<u64>> for Threads {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 2 {
            return Err(format!("invalid Threads line {v:?}").into());
        }

        Ok(Self {
            threads: v[0],
            full_cnt: v[1],
        })
    }
}

// ReadAheadCache models the "ra" line.
#[derive(Debug, Default, PartialEq)]
struct ReadAheadCache {
    cache_size: u64,
    cache_histogram: Vec<u64>,
    not_found: u64,
}

impl TryFrom<Vec<u64>> for ReadAheadCache {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 12 {
            return Err(format!("invalid ReadAheadCache line {v:?}").into());
        }

        Ok(Self {
            cache_size: v[0],
            cache_histogram: v[1..11].to_vec(),
            not_found: v[11],
        })
    }
}

// ServerRPC models the nfsd "rpc" line.
#[derive(Debug, Default, PartialEq)]
struct ServerRPC {
    rpc_count: u64,
    bad_cnt: u64,
    bad_fmt: u64,
    bad_auth: u64,
    badc_int: u64,
}

impl TryFrom<Vec<u64>> for ServerRPC {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 5 {
            return Err(Error::Malformed("server RPC values"));
        }

        Ok(Self {
            rpc_count: v[0],
            bad_cnt: v[1],
            bad_fmt: v[2],
            bad_auth: v[3],
            badc_int: v[4],
        })
    }
}

// ServerV4Stats models the nfsd "proc4" line.
#[derive(Debug, Default, PartialEq)]
struct ServerV4Stats {
    null: u64,
    compound: u64,
}

impl TryFrom<Vec<u64>> for ServerV4Stats {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        let vs = v[0] as usize;
        if v.len() - 1 != vs || vs != 2 {
            return Err(format!("invalid V4Stats line {v:?}").into());
        }

        Ok(Self {
            null: v[1],
            compound: v[2],
        })
    }
}

// V4Ops models the "proc4ops" line: NFSv4 operations
// Variable list, see:
// v4.0 https://tools.ietf.org/html/rfc3010 (38 operations)
// v4.1 https://tools.ietf.org/html/rfc5661 (58 operations)
// v4.2 https://tools.ietf.org/html/draft-ietf-nfsv4-minorversion2-41 (71 operations)
#[derive(Debug, Default, PartialEq)]
struct V4Ops {
    // values:       u64 // Variable depending on v4.x sub-version. TODO: Will this always at least include the fields in this struct?
    op0_unused: u64,
    op1_unused: u64,
    op2_future: u64,
    access: u64,
    close: u64,
    commit: u64,
    create: u64,
    deleg_purge: u64,
    deleg_return: u64,
    get_attr: u64,
    get_fh: u64,
    link: u64,
    lock: u64,
    lockt: u64,
    locku: u64,
    lookup: u64,
    lookup_root: u64,
    nverify: u64,
    open: u64,
    open_attr: u64,
    open_confirm: u64,
    open_dgrd: u64,
    put_fh: u64,
    put_pub_fh: u64,
    put_root_fh: u64,
    read: u64,
    readdir: u64,
    read_link: u64,
    remove: u64,
    rename: u64,
    renew: u64,
    restore_fh: u64,
    save_fh: u64,
    sec_info: u64,
    set_attr: u64,
    set_client_id: u64,
    set_client_id_confirm: u64,
    verify: u64,
    write: u64,
    rel_lock_owner: u64,
}

impl TryFrom<Vec<u64>> for V4Ops {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        let vs = v[0] as usize;
        if v.len() - 1 != vs || vs < 39 {
            return Err(format!("invalid V4Ops line {v:?}").into());
        }

        Ok(Self {
            op0_unused: v[1],
            op1_unused: v[2],
            op2_future: v[3],
            access: v[4],
            close: v[5],
            commit: v[6],
            create: v[7],
            deleg_purge: v[8],
            deleg_return: v[9],
            get_attr: v[10],
            get_fh: v[11],
            link: v[12],
            lock: v[13],
            lockt: v[14],
            locku: v[15],
            lookup: v[16],
            lookup_root: v[17],
            nverify: v[18],
            open: v[19],
            open_attr: v[20],
            open_confirm: v[21],
            open_dgrd: v[22],
            put_fh: v[23],
            put_pub_fh: v[24],
            put_root_fh: v[25],
            read: v[26],
            readdir: v[27],
            read_link: v[28],
            remove: v[29],
            rename: v[30],
            renew: v[31],
            restore_fh: v[32],
            save_fh: v[33],
            sec_info: v[34],
            set_attr: v[35],
            set_client_id: v[36],
            set_client_id_confirm: v[37],
            verify: v[38],
            write: v[39],
            rel_lock_owner: if v[0] > 39 { v[40] } else { 0 },
        })
    }
}

#[derive(Debug, Default, PartialEq)]
struct ServerRPCStats {
    reply_cache: ReplyCache,
    file_handles: FileHandles,
    input_output: InputOutput,
    threads: Threads,
    read_ahead_cache: ReadAheadCache,
    network: Network,
    server_rpc: ServerRPC,
    v2_stats: V2Stats,
    v3_stats: V3Stats,
    server_v4_stats: ServerV4Stats,
    v4_ops: V4Ops,
    wdeleg_getattr: u64,
}

fn load_server_rpc_stats<P: AsRef<Path>>(path: P) -> Result<ServerRPCStats, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut stats = ServerRPCStats::default();
    for line in data.lines() {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            return Err(format!("invalid NFSd metric line {line}").into());
        }

        let label = parts[0];
        let values = match label {
            "th" => {
                if parts.len() < 3 {
                    return Err(format!("invalid NFSd th metric line {line}").into());
                }

                // TODO: handle the parse error
                parts[1..3]
                    .iter()
                    .map(|p| (*p).parse::<u64>().unwrap_or(0))
                    .collect::<Vec<_>>()
            }
            _ => parts[1..]
                .iter()
                .map(|p| (*p).parse::<u64>().unwrap_or(0))
                .collect::<Vec<_>>(),
        };

        let metric_line = parts[0];
        match metric_line {
            "rc" => stats.reply_cache = values.try_into()?,
            "fh" => stats.file_handles = values.try_into()?,
            "io" => stats.input_output = values.try_into()?,
            "th" => stats.threads = values.try_into()?,
            "ra" => stats.read_ahead_cache = values.try_into()?,
            "net" => stats.network = values.try_into()?,
            "rpc" => stats.server_rpc = values.try_into()?,
            "proc2" => stats.v2_stats = values.try_into()?,
            "proc3" => stats.v3_stats = values.try_into()?,
            "proc4" => stats.server_v4_stats = values.try_into()?,
            "proc4ops" => stats.v4_ops = values.try_into()?,
            "wdeleg_getattr" => stats.wdeleg_getattr = values[0],
            _ => return Err(format!("errors parsing NFSd metric line {line}").into()),
        }
    }

    Ok(stats)
}

macro_rules! rpc_metric {
    ($proto: expr, $name: expr, $value: expr) => {
        Metric::sum_with_tags(
            "node_nfsd_requests_total",
            "Total number NFSd Requests by method and protocol",
            $value,
            tags! {
                "proto" => $proto,
                "method" => $name
            },
        )
    };
}

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = load_server_rpc_stats(proc_path.join("net/rpc/nfsd"))?;
    let metrics = vec![
        // collects statistics for the reply cache
        Metric::sum(
            "node_nfsd_reply_cache_hits_total",
            "Total number of NFSd Reply Cache hits (client lost server response).",
            stats.reply_cache.hits,
        ),
        Metric::sum(
            "node_nfsd_reply_cache_misses_total",
            "Total number of NFSd Reply Cache an operation that requires caching (idempotent).",
            stats.reply_cache.misses,
        ),
        Metric::sum(
            "node_nfsd_reply_cache_nocache_total",
            "Total number of NFSd Reply Cache non-idempotent operations (rename/delete/…).",
            stats.reply_cache.no_cache,
        ),
        // collects statistics for the file handles
        // NOTE: Other FileHandles entries are unused in the kernel
        Metric::sum(
            "node_nfsd_file_handles_stale_total",
            "Total number of NFSd stale file handles",
            stats.file_handles.stale,
        ),
        // collects statistics for the bytes in/out
        Metric::sum(
            "node_nfsd_disk_bytes_read_total",
            "Total NFSd bytes read.",
            stats.input_output.read,
        ),
        Metric::sum(
            "node_nfsd_disk_bytes_written_total",
            "Total NFSd bytes written.",
            stats.input_output.write,
        ),
        // collects statistics for kernel server threads
        Metric::gauge(
            "node_nfsd_server_threads",
            "Total number of NFSd kernel threads that are running.",
            stats.threads.threads,
        ),
        // collects statistics for the read ahead cache.
        Metric::gauge(
            "node_nfsd_read_ahead_cache_size_blocks",
            "How large the read ahead cache is in blocks.",
            stats.read_ahead_cache.cache_size,
        ),
        Metric::sum(
            "node_nfsd_read_ahead_cache_not_found_total",
            "Total number of NFSd read ahead cache not found.",
            stats.read_ahead_cache.not_found,
        ),
        // collects statistics for network packets/connections.
        Metric::sum_with_tags(
            "node_nfsd_packets_total",
            "Total NFSd network packets (sent+received) by protocol type.",
            stats.network.udp_count,
            tags!(
                "proto" => "udp"
            ),
        ),
        Metric::sum_with_tags(
            "node_nfsd_packets_total",
            "Total NFSd network packets (sent+received) by protocol type.",
            stats.network.tcp_count,
            tags!(
                "proto" => "tcp"
            ),
        ),
        Metric::sum(
            "node_nfsd_connections_total",
            "Total number of NFSd TCP connections.",
            stats.network.tcp_connect,
        ),
        // collects statistics for kernel server RPCs.
        Metric::sum_with_tags(
            "node_nfsd_rpc_errors_total",
            "Total number of NFSd RPC errors by error type.",
            stats.server_rpc.bad_fmt,
            tags!(
                "error" => "fmt"
            ),
        ),
        Metric::sum_with_tags(
            "node_nfsd_rpc_errors_total",
            "Total number of NFSd RPC errors by error type.",
            stats.server_rpc.bad_auth,
            tags!(
                "error" => "auth"
            ),
        ),
        Metric::sum_with_tags(
            "node_nfsd_rpc_errors_total",
            "Total number of NFSd RPC errors by error type.",
            stats.server_rpc.badc_int,
            tags!(
                "error" => "cInt"
            ),
        ),
        Metric::sum(
            "node_nfsd_server_rpcs_total",
            "Total number of NFSd RPCs.",
            stats.server_rpc.rpc_count,
        ),
        // collects statistics for NFSv2 requests
        rpc_metric!("2", "GetAttr", stats.v2_stats.get_attr),
        rpc_metric!("2", "SetAttr", stats.v2_stats.set_attr),
        rpc_metric!("2", "Root", stats.v2_stats.root),
        rpc_metric!("2", "Lookup", stats.v2_stats.lookup),
        rpc_metric!("2", "ReadLink", stats.v2_stats.read_link),
        rpc_metric!("2", "Read", stats.v2_stats.read),
        rpc_metric!("2", "WrCache", stats.v2_stats.wr_cache),
        rpc_metric!("2", "Write", stats.v2_stats.write),
        rpc_metric!("2", "Create", stats.v2_stats.create),
        rpc_metric!("2", "Remove", stats.v2_stats.remove),
        rpc_metric!("2", "Rename", stats.v2_stats.rename),
        rpc_metric!("2", "Link", stats.v2_stats.link),
        rpc_metric!("2", "SymLink", stats.v2_stats.sym_link),
        rpc_metric!("2", "MkDir", stats.v2_stats.mkdir),
        rpc_metric!("2", "RmDir", stats.v2_stats.rmdir),
        rpc_metric!("2", "ReadDir", stats.v2_stats.read_dir),
        rpc_metric!("2", "FsStat", stats.v2_stats.fs_stat),
        // collects statistics for NFSv3 requests
        rpc_metric!("3", "GetAttr", stats.v3_stats.get_attr),
        rpc_metric!("3", "SetAttr", stats.v3_stats.set_attr),
        rpc_metric!("3", "Lookup", stats.v3_stats.lookup),
        rpc_metric!("3", "Access", stats.v3_stats.access),
        rpc_metric!("3", "ReadLink", stats.v3_stats.read_link),
        rpc_metric!("3", "Read", stats.v3_stats.read),
        rpc_metric!("3", "Write", stats.v3_stats.write),
        rpc_metric!("3", "Create", stats.v3_stats.create),
        rpc_metric!("3", "MkDir", stats.v3_stats.mkdir),
        rpc_metric!("3", "SymLink", stats.v3_stats.sym_link),
        rpc_metric!("3", "MkNod", stats.v3_stats.mknod),
        rpc_metric!("3", "Remove", stats.v3_stats.remove),
        rpc_metric!("3", "RmDir", stats.v3_stats.rmdir),
        rpc_metric!("3", "Rename", stats.v3_stats.rename),
        rpc_metric!("3", "Link", stats.v3_stats.link),
        rpc_metric!("3", "ReadDir", stats.v3_stats.read_dir),
        rpc_metric!("3", "ReadDirPlus", stats.v3_stats.read_dir_plus),
        rpc_metric!("3", "FsStat", stats.v3_stats.fs_stat),
        rpc_metric!("3", "FsInfo", stats.v3_stats.fs_info),
        rpc_metric!("3", "PathConf", stats.v3_stats.path_conf),
        rpc_metric!("3", "Commit", stats.v3_stats.commit),
        // collects statistics for NFSv4 requests
        rpc_metric!("4", "Access", stats.v4_ops.access),
        rpc_metric!("4", "Close", stats.v4_ops.close),
        rpc_metric!("4", "Commit", stats.v4_ops.commit),
        rpc_metric!("4", "Create", stats.v4_ops.create),
        rpc_metric!("4", "DelegPurge", stats.v4_ops.deleg_purge),
        rpc_metric!("4", "DelegReturn", stats.v4_ops.deleg_return),
        rpc_metric!("4", "GetAttr", stats.v4_ops.get_attr),
        rpc_metric!("4", "GetFH", stats.v4_ops.get_fh),
        rpc_metric!("4", "Link", stats.v4_ops.link),
        rpc_metric!("4", "Lock", stats.v4_ops.lock),
        rpc_metric!("4", "Lockt", stats.v4_ops.lockt),
        rpc_metric!("4", "Locku", stats.v4_ops.locku),
        rpc_metric!("4", "Lookup", stats.v4_ops.lookup),
        rpc_metric!("4", "LookupRoot", stats.v4_ops.lookup_root),
        rpc_metric!("4", "Nverify", stats.v4_ops.nverify),
        rpc_metric!("4", "Open", stats.v4_ops.open),
        rpc_metric!("4", "OpenAttr", stats.v4_ops.open_attr),
        rpc_metric!("4", "OpenConfirm", stats.v4_ops.open_confirm),
        rpc_metric!("4", "OpenDgrd", stats.v4_ops.open_dgrd),
        rpc_metric!("4", "PutFH", stats.v4_ops.put_fh),
        rpc_metric!("4", "Read", stats.v4_ops.read),
        rpc_metric!("4", "ReadDir", stats.v4_ops.readdir),
        rpc_metric!("4", "ReadLink", stats.v4_ops.read_link),
        rpc_metric!("4", "Remove", stats.v4_ops.remove),
        rpc_metric!("4", "Rename", stats.v4_ops.rename),
        rpc_metric!("4", "Renew", stats.v4_ops.renew),
        rpc_metric!("4", "RestoreFH", stats.v4_ops.restore_fh),
        rpc_metric!("4", "SaveFH", stats.v4_ops.save_fh),
        rpc_metric!("4", "SecInfo", stats.v4_ops.sec_info),
        rpc_metric!("4", "SetAttr", stats.v4_ops.set_attr),
        rpc_metric!("4", "SetClientID", stats.v4_ops.set_client_id),
        rpc_metric!(
            "4",
            "SetClientIDConfirm",
            stats.v4_ops.set_client_id_confirm
        ),
        rpc_metric!("4", "Verify", stats.v4_ops.verify),
        rpc_metric!("4", "Write", stats.v4_ops.write),
        rpc_metric!("4", "RelLockOwner", stats.v4_ops.rel_lock_owner),
        rpc_metric!("4", "WdelegGetattr", stats.wdeleg_getattr),
    ];

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testify::temp_dir;

    #[test]
    fn server_rpc_stats() {
        let cases = vec![
            (
                "invalid file",
                "invalid",
                Default::default(),
                true,
            ),
            (
                "good file",
                "rc 0 6 18622
fh 0 0 0 0 0
io 157286400 0
th 8 0 0.000 0.000 0.000 0.000 0.000 0.000 0.000 0.000 0.000 0.000
ra 32 0 0 0 0 0 0 0 0 0 0 0
net 18628 0 18628 6
rpc 18628 0 0 0 0
proc2 18 2 69 0 0 4410 0 0 0 0 0 0 0 0 0 0 0 99 2
proc3 22 2 112 0 2719 111 0 0 0 0 0 0 0 0 0 0 0 27 216 0 2 1 0
proc4 2 2 10853
proc4ops 72 0 0 0 1098 2 0 0 0 0 8179 5896 0 0 0 0 5900 0 0 2 0 2 0 9609 0 2 150 1272 0 0 0 1236 0 0 0 0 3 3 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
wdeleg_getattr 16",
                ServerRPCStats {
                    reply_cache: ReplyCache {
                        hits: 0,
                        misses: 6,
                        no_cache: 18622,
                    },
                    file_handles: FileHandles {
                        stale: 0,
                        total_lookups: 0,
                        anon_lookups: 0,
                        dir_no_cache: 0,
                        no_dir_no_cache: 0,
                    },
                    input_output: InputOutput {
                        read: 157286400,
                        write: 0,
                    },
                    threads: Threads {
                        threads: 8,
                        full_cnt: 0,
                    },
                    read_ahead_cache: ReadAheadCache {
                        cache_size: 32,
                        cache_histogram: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                        not_found: 0,
                    },
                    network: Network {
                        net_count: 18628,
                        udp_count: 0,
                        tcp_count: 18628,
                        tcp_connect: 6,
                    },
                    server_rpc: ServerRPC {
                        rpc_count: 18628,
                        bad_cnt: 0,
                        bad_fmt: 0,
                        bad_auth: 0,
                        badc_int: 0,
                    },
                    v2_stats: V2Stats {
                        null: 2,
                        get_attr: 69,
                        set_attr: 0,
                        root: 0,
                        lookup: 4410,
                        read_link: 0,
                        read: 0,
                        wr_cache: 0,
                        write: 0,
                        create: 0,
                        remove: 0,
                        rename: 0,
                        link: 0,
                        sym_link: 0,
                        mkdir: 0,
                        rmdir: 0,
                        read_dir: 99,
                        fs_stat: 2,
                    },
                    v3_stats: V3Stats {
                        null: 2,
                        get_attr: 112,
                        set_attr: 0,
                        lookup: 2719,
                        access: 111,
                        read_link: 0,
                        read: 0,
                        write: 0,
                        create: 0,
                        mkdir: 0,
                        sym_link: 0,
                        mknod: 0,
                        remove: 0,
                        rmdir: 0,
                        rename: 0,
                        link: 0,
                        read_dir: 27,
                        read_dir_plus: 216,
                        fs_stat: 0,
                        fs_info: 2,
                        path_conf: 1,
                        commit: 0,
                    },
                    server_v4_stats: ServerV4Stats {
                        null: 2,
                        compound: 10853,
                    },
                    v4_ops: V4Ops {
                        op0_unused: 0,
                        op1_unused: 0,
                        op2_future: 0,
                        access: 1098,
                        close: 2,
                        commit: 0,
                        create: 0,
                        deleg_purge: 0,
                        deleg_return: 0,
                        get_attr: 8179,
                        get_fh: 5896,
                        link: 0,
                        lock: 0,
                        lockt: 0,
                        locku: 0,
                        lookup: 5900,
                        lookup_root: 0,
                        nverify: 0,
                        open: 2,
                        open_attr: 0,
                        open_confirm: 2,
                        open_dgrd: 0,
                        put_fh: 9609,
                        put_pub_fh: 0,
                        put_root_fh: 2,
                        read: 150,
                        readdir: 1272,
                        read_link: 0,
                        remove: 0,
                        rename: 0,
                        renew: 1236,
                        restore_fh: 0,
                        save_fh: 0,
                        sec_info: 0,
                        set_attr: 0,
                        set_client_id: 3,
                        set_client_id_confirm: 3,
                        verify: 0,
                        write: 0,
                        rel_lock_owner: 0,
                    },
                    wdeleg_getattr: 16
                },
                false
            ),
        ];

        let tmpdir = temp_dir();
        for (name, input, stats, valid) in cases {
            let path = tmpdir.join(name);
            std::fs::write(&path, input).unwrap();
            let result = load_server_rpc_stats(&path);
            if valid && result.is_err() {
                continue;
            }

            assert_eq!(result.unwrap(), stats);
        }
    }
}
