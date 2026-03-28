use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::time::Duration;

use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = match std::fs::read_to_string(paths.proc().join("self/mountstats")) {
        Ok(content) => content,
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return Err(Error::NoData);
            }

            return Err(Error::from(err));
        }
    };
    let mounts = parse_mount_stats(&content)?;

    let content = read_file_no_stat(paths.proc().join("self/mountinfo"))?;
    let infos = parse_mount_info(&content)?;

    let mut seen = BTreeSet::new();
    let mut metrics = Vec::with_capacity(mounts.len());
    for (index, mount) in mounts.into_iter().enumerate() {
        // The mount entry order in the /proc/self/mountstats and /proc/self/mountinfo is the same
        let addr = if index < infos.len() {
            infos[index]
                .super_options
                .split(',')
                .find_map(|part| part.strip_prefix("addr="))
                .unwrap_or("")
        } else {
            ""
        };

        for transport in &mount.stats.transports {
            if !seen.insert((mount.device, transport.protocol, addr)) {
                debug!(
                    message = "skipping duplicate device entry",
                    device = mount.device,
                    protocol = transport.protocol,
                    addr
                );

                break;
            }

            metrics.extend(build_nfs_metrics(
                &mount.stats,
                mount.device,
                transport.protocol,
                addr,
            ));
        }
    }

    Ok(metrics)
}

fn build_nfs_metrics(stat: &MountStats, device: &str, protocol: &str, addr: &str) -> Vec<Metric> {
    // 64-bit float mantissa: https://en.wikipedia.org/wiki/Double-precision_floating-point_format
    const FLOAT64_MANTISSA: u64 = 9007199254740992;

    let tags = tags!(
        "export" => device,
        "mountaddr" => addr,
        "protocol" => protocol,
    );

    let mut metrics = vec![
        Metric::sum_with_tags(
            "node_mountstats_nfs_age_seconds_total",
            "The age of the NFS mount in seconds.",
            stat.age,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_read_bytes_total",
            "Number of bytes read using the read() syscall.",
            stat.bytes.read,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_write_bytes_total",
            "Number of bytes written using the write() syscall.",
            stat.bytes.write,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_direct_read_bytes_total",
            "Number of bytes read using the read() syscall in O_DIRECT mode.",
            stat.bytes.direct_read,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_direct_write_bytes_total",
            "Number of bytes written using the write() syscall in O_DIRECT mode.",
            stat.bytes.direct_write,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_total_read_bytes_total",
            "Number of bytes read from the NFS server, in total.",
            stat.bytes.read_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_total_write_bytes_total",
            "Number of bytes written to the NFS server, in total.",
            stat.bytes.write_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_read_pages_total",
            "Number of pages read directly via mmap()'d files.",
            stat.bytes.read_pages,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_write_pages_total",
            "Number of pages written directly via mmap()'d files.",
            stat.bytes.write_pages,
            tags.clone(),
        ),
    ];

    for (index, transport) in stat.transports.iter().enumerate() {
        let tags = tags!(
            "export" => device,
            "mountaddr" => addr,
            "protocol" => protocol,
            "transport" => index,
        );

        metrics.extend([
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_bind_total",
                "Number of times the client has had to establish a connection from scratch to the NFS server.",
                transport.bind,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_connect_total",
                "Number of times the client has made a TCP connection to the NFS server.",
                transport.connect,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_mountstats_nfs_transport_idle_time_seconds",
                "Duration since the NFS mount last saw any RPC traffic, in seconds.",
                transport.idle_time_seconds % FLOAT64_MANTISSA,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_sends_total",
                "Number of RPC requests for this mount sent to the NFS server.",
                transport.sends,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_receives_total",
                "Number of RPC responses for this mount received from the NFS server.",
                transport.receives,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_bad_transaction_ids_total",
                "Number of times the NFS server sent a response with a transaction ID unknown to this client.",
                transport.bad_transaction_ids,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_backlog_queue_total",
                "Total number of items added to the RPC backlog queue.",
                transport.cumulative_backlog,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_mountstats_nfs_transport_maximum_rpc_slots",
                "Maximum number of simultaneously active RPC requests ever used.",
                transport.maximum_rpc_slots_used,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_sending_queue_total",
                "Total number of items added to the RPC transmission sending queue.",
                transport.cumulative_sending_queue,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_transport_pending_queue_total",
                "Total number of items added to the RPC transmission pending queue.",
                transport.cumulative_pending_queue,
                tags
            )
        ]);
    }

    for operation in &stat.operations {
        let tags = tags!(
            "export" => device,
            "mountaddr" => addr,
            "operation" => operation.operation,
            "protocol" => protocol,
        );

        metrics.extend([
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_requests_total",
                "Number of requests performed for a given operation.",
                operation.requests,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_transmissions_total",
                "Number of times an actual RPC request has been transmitted for a given operation.",
                operation.transmissions,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_major_timeouts_total",
                "Number of times a request has had a major timeout for a given operation.",
                operation.major_timeouts,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_sent_bytes_total",
                "Number of bytes sent for a given operation, including RPC headers and payload.",
                operation.bytes_sent,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_received_bytes_total",
                "Number of bytes received for a given operation, including RPC headers and payload.",
                operation.bytes_received,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_queue_time_seconds_total",
                "Duration all requests spent queued for transmission for a given operation before they were sent, in seconds.",
                (operation.cumulative_queue_milliseconds % FLOAT64_MANTISSA) as f64 / 1000.0,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_response_time_seconds_total",
                "Duration all requests took to get a reply back after a request for a given operation was transmitted, in seconds.",
                (operation.cumulative_total_response_milliseconds % FLOAT64_MANTISSA) as f64 / 1000.0,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_mountstats_nfs_operations_request_time_seconds_total",
                "Duration all requests took from when a request was enqueued to when it was completely handled for a given operation, in seconds.",
                (operation.cumulative_total_request_milliseconds % FLOAT64_MANTISSA) as f64 / 1000.0,
                tags,
            ),
        ])
    }

    // event statistics
    metrics.extend([
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_inode_revalidate_total",
            "Number of times cached inode attributes are re-validated from the server.",
            stat.events.inode_revalidate,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_dnode_revalidate_total",
            "Number of times cached dentry nodes are re-validated from the server.",
            stat.events.dnode_revalidate,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_data_invalidate_total",
            "Number of times an inode cache is cleared.",
            stat.events.data_invalidate,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_attribute_invalidate_total",
            "Number of times cached inode attributes are invalidated.",
            stat.events.attribute_invalidate,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_open_total",
            "Number of times cached inode attributes are invalidated.",
            stat.events.vfs_open,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_lookup_total",
            "Number of times a directory lookup has occurred.",
            stat.events.vfs_lookup,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_access_total",
            "Number of times permissions have been checked.",
            stat.events.vfs_access,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_update_page_total",
            "Number of updates (and potential writes) to pages.",
            stat.events.vfs_update_page,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_read_page_total",
            "Number of pages read directly via mmap()'d files.",
            stat.events.vfs_read_page,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_read_pages_total",
            "Number of times a group of pages have been read.",
            stat.events.vfs_read_pages,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_write_page_total",
            "Number of pages written directly via mmap()'d files.",
            stat.events.vfs_write_page,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_write_pages_total",
            "Number of times a group of pages have been written.",
            stat.events.vfs_write_pages,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_getdents_total",
            "Number of times directory entries have been read with getdents().",
            stat.events.vfs_getdents,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_setattr_total",
            "Number of times directory entries have been read with getdents().",
            stat.events.vfs_setattr,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_flush_total",
            "Number of pending writes that have been forcefully flushed to the server.",
            stat.events.vfs_flush,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_fsync_total",
            "Number of times fsync() has been called on directories and files.",
            stat.events.vfs_fsync,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_lock_total",
            "Number of times locking has been attempted on a file.",
            stat.events.vfs_lock,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_vfs_file_release_total",
            "Number of times files have been closed and released.",
            stat.events.vfs_file_release,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_truncation_total",
            "Number of times files have been truncated.",
            stat.events.truncation,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_write_extension_total",
            "Number of times a file has been grown due to writes beyond its existing end.",
            stat.events.write_extension,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_silly_rename_total",
            "Number of times a file was removed while still open by another process.",
            stat.events.silly_rename,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_short_read_total",
            "Number of times the NFS server gave less data than expected while reading.",
            stat.events.short_read,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_short_write_total",
            "Number of times the NFS server wrote less data than expected while writing.",
            stat.events.short_write,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_jukebox_delay_total",
            "Number of times the NFS server indicated EJUKEBOX; retrieving data from offline storage.",
            stat.events.jukebox_delay,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_pnfs_read_total",
            "Number of NFS v4.1+ pNFS reads.",
            stat.events.pnfs_read,
            tags.clone()
        ),
        Metric::sum_with_tags(
            "node_mountstats_nfs_event_pnfs_write_total",
            "Number of NFS v4.1+ pNFS writes.",
            stat.events.pnfs_write,
            tags
        ),
    ]);

    metrics
}

// Statistics about the number of bytes read and written by an NFS
// client to and from an NFS server
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct BytesStats {
    // Number of bytes read using the read() syscall
    read: u64,
    // Number of bytes written using the write() syscall
    write: u64,
    // Number of bytes read suing the read() syscall in O_DIRECT mode
    direct_read: u64,
    // Number of bytes written using the write() syscall in O_DIRECT mode
    direct_write: u64,
    // Number of bytes read from the NFS server, in total
    read_total: u64,
    // Number of bytes written to the NFS server, in total.
    write_total: u64,
    // Number of pages read directly via mmap()'d files.
    read_pages: u64,
    // Number of pages written directly via mmap()'d files.
    write_pages: u64,
}

// statistics about NFS event occurrences.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct EventsStats {
    // Number of times cached inode attributes are re-validated from the server.
    inode_revalidate: u64,
    // Number of times cached dentry nodes are re-validated from the server.
    dnode_revalidate: u64,
    // Number of times an inode cache is cleared.
    data_invalidate: u64,
    // Number of times cached inode attributes are invalidated.
    attribute_invalidate: u64,
    // Number of times files or directories have been open()'d.
    vfs_open: u64,
    // Number of times a directory lookup has occurred.
    vfs_lookup: u64,
    // Number of times permissions have been checked.
    vfs_access: u64,
    // Number of updates (and potential writes) to pages.
    vfs_update_page: u64,
    // Number of pages read directly via mmap()'d files.
    vfs_read_page: u64,
    // Number of times a group of pages have been read.
    vfs_read_pages: u64,
    // Number of pages written directly via mmap()'d files.
    vfs_write_page: u64,
    // Number of times a group of pages have been written.
    vfs_write_pages: u64,
    // Number of times directory entries have been read with getdents().
    vfs_getdents: u64,
    // Number of times attributes have been set on inodes.
    vfs_setattr: u64,
    // Number of pending writes that have been forcefully flushed to the server.
    vfs_flush: u64,
    // Number of times fsync() has been called on directories and files.
    vfs_fsync: u64,
    // Number of times locking has been attempted on a file.
    vfs_lock: u64,
    // Number of times files have been closed and released.
    vfs_file_release: u64,
    // Unknown.  Possibly unused.
    congestion_wait: u64,
    // Number of times files have been truncated.
    truncation: u64,
    // Number of times a file has been grown due to writes beyond its existing end.
    write_extension: u64,
    // Number of times a file was removed while still open by another process.
    silly_rename: u64,
    // Number of times the NFS server gave less data than expected while reading.
    short_read: u64,
    // Number of times the NFS server wrote less data than expected while writing.
    short_write: u64,
    // Number of times the NFS server indicated EJUKEBOX; retrieving data from
    // offline storage.
    jukebox_delay: u64,
    // Number of NFS v4.1+ pNFS reads.
    pnfs_read: u64,
    // Number of NFS v4.1+ pNFS writes.
    pnfs_write: u64,
}

// statistics for a single operation
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
struct OperationStats<'a> {
    operation: &'a str,
    // Number of requests performed for this operation.
    requests: u64,
    // Number of times an actual RPC request has been transmitted for this operation.
    transmissions: u64,
    // Number of times a request has had a major timeout.
    major_timeouts: u64,
    // Number of bytes sent for this operation, including RPC headers and payload.
    bytes_sent: u64,
    // Number of bytes received for this operation, including RPC headers and payload.
    bytes_received: u64,
    // Duration all requests spent queued for transmission before they were sent.
    cumulative_queue_milliseconds: u64,
    // Duration it took to get a reply back after the request was transmitted.
    cumulative_total_response_milliseconds: u64,
    // Duration from when a request was enqueued to when it was completely handled.
    cumulative_total_request_milliseconds: u64,
    // The count of operations that complete with tk_status < 0.  These statuses usually indicate error conditions.
    errors: u64,
}

// statistics for the NFS mount RPC requests and response
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
struct TransportStats<'a> {
    // The transport protocol used for the NFS mount.
    protocol: &'a str,

    // Number of times the client has had to establish a connection from scratch
    // to the NFS server.
    bind: u64,
    // Number of times the client has made a TCP connection to the NFS server.
    connect: u64,
    // Duration (in jiffies, a kernel internal unit of time) the NFS mount has
    // spent waiting for connections to the server to be established.
    connect_idle_time: u64,
    // Duration since the NFS mount last saw any RPC traffic.
    idle_time_seconds: u64,
    // Number of RPC requests for this mount sent to the NFS server.
    sends: u64,
    // Number of RPC responses for this mount received from the NFS server.
    receives: u64,
    // Number of times the NFS server sent a response with a transaction ID
    // unknown to this client.
    bad_transaction_ids: u64,
    // A running counter, incremented on each request as the current difference
    // between sends and receives.
    cumulative_active_requests: u64,
    // A running counter, incremented on each request by the current backlog
    // queue size.
    cumulative_backlog: u64,

    // Stats below only available with stat version 1.1.

    // Maximum number of simultaneously active RPC requests ever used.
    maximum_rpc_slots_used: u64,
    // A running counter, incremented on each request as the current size of the
    // sending queue.
    cumulative_sending_queue: u64,
    // A running counter, incremented on each request as the current size of the
    // pending queue.
    cumulative_pending_queue: u64,
}

// Stat for NFSv3 and v4 mounts
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct MountStats<'a> {
    // the version of statistics provided
    version: &'a str,
    // the mount options of the NFS mount
    opts: &'a str,
    // the age of the NFS mount
    age: Duration,
    // byte counters for various operations
    bytes: BytesStats,
    // Statistics related to various NFS event occurrences.
    events: EventsStats,
    // statistics broken down by filesystem operation
    operations: Vec<OperationStats<'a>>,
    // statistics about the NFS RPC transport
    transports: Vec<TransportStats<'a>>,
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
struct Mount<'a> {
    // name of the device
    device: &'a str,
    // the mount point of the device
    mount: &'a str,
    // the filesystem type used by the device
    typ: &'a str,

    // if available additional statistics related to this Mount.
    stats: MountStats<'a>,
}

// A MountInfo is a type that describes the details, options for each
// mount, parsed from /proc/self/mountinfo.
// The fields described in each entry of /proc/self/mountinfo
// is described in the following man page
// http://man7.org/linux/man-pages/man5/proc.5.html
struct MountInfo<'a> {
    super_options: &'a str,
}

fn parse_mount_stats(content: &str) -> Result<Vec<Mount<'_>>, Error> {
    let mut lines = content.lines();
    let mut mounts = Vec::new();

    loop {
        let Some(line) = lines.next() else {
            break;
        };

        let mut fields = line.split_ascii_whitespace();

        // check for specific words appearing at specific indices to ensure
        // the format is consistent with what we expect
        //
        // e.g.  device [device] mounted on [mount] with fstype [type]
        if fields.next() != Some("device") {
            continue;
        }
        let Some(device) = fields.next() else {
            return Err(Error::Malformed("device line"));
        };

        if fields.next() != Some("mounted") {
            return Err(Error::Malformed("device line"));
        }
        if fields.next() != Some("on") {
            return Err(Error::Malformed("device line"));
        }
        let Some(mount) = fields.next() else {
            return Err(Error::Malformed("device line"));
        };

        if fields.next() != Some("with") {
            return Err(Error::Malformed("device line"));
        }
        if fields.next() != Some("fstype") {
            return Err(Error::Malformed("device line"));
        }
        let Some(typ) = fields.next() else {
            return Err(Error::Malformed("device line"));
        };

        // does this mount also possess statistics information?
        if let Some(field) = fields.next() {
            // Only NFSv3 and v4 are supported for parsing statistics
            if typ != "nfs" && typ != "nfs4" {
                return Err(Error::Malformed("device line"));
            }

            let Some(version) = field.strip_prefix("statvers=") else {
                return Err(Error::Malformed("statvers field"));
            };
            let stats = parse_nfs_mount_stats(&mut lines, version)?;

            mounts.push(Mount {
                device,
                mount,
                typ,
                stats,
            });
        } else {
            mounts.push(Mount {
                device,
                mount,
                typ,
                stats: Default::default(),
            })
        }
    }

    Ok(mounts)
}

fn parse_nfs_mount_stats<'a, I: Iterator<Item = &'a str>>(
    mut lines: I,
    version: &'a str,
) -> Result<MountStats<'a>, Error> {
    let mut stat = MountStats {
        version,
        ..Default::default()
    };

    loop {
        let Some(line) = lines.next() else {
            break;
        };

        let mut fields = line.split_ascii_whitespace();
        let Some(field) = fields.next() else {
            continue;
        };

        match field {
            "opts:" => {
                let Some(value) = fields.next() else {
                    return Err(Error::Malformed("NFS stats"));
                };

                stat.opts = value;
            }
            "age:" => {
                let Some(value) = fields.next() else {
                    return Err(Error::Malformed("NFS stats"));
                };

                let secs = value.parse::<u64>()?;
                stat.age = Duration::from_secs(secs);
            }
            "bytes:" => {
                let values = fields
                    .take(8)
                    .map(|field| field.parse::<u64>())
                    .collect::<Result<Vec<_>, _>>()?;
                if values.len() != 8 {
                    return Err(Error::Malformed("NFS stats"));
                }

                stat.bytes.read = values[0];
                stat.bytes.write = values[1];
                stat.bytes.direct_read = values[2];
                stat.bytes.direct_write = values[3];
                stat.bytes.read_total = values[4];
                stat.bytes.write_total = values[5];
                stat.bytes.read_pages = values[6];
                stat.bytes.write_pages = values[7];
            }
            "events:" => {
                let values = fields
                    .take(27)
                    .map(|field| field.parse::<u64>())
                    .collect::<Result<Vec<_>, _>>()?;
                if values.len() != 27 {
                    return Err(Error::Malformed("NFS stats"));
                }

                stat.events.inode_revalidate = values[0];
                stat.events.dnode_revalidate = values[1];
                stat.events.data_invalidate = values[2];
                stat.events.attribute_invalidate = values[3];
                stat.events.vfs_open = values[4];
                stat.events.vfs_lookup = values[5];
                stat.events.vfs_access = values[6];
                stat.events.vfs_update_page = values[7];
                stat.events.vfs_read_page = values[8];
                stat.events.vfs_read_pages = values[9];
                stat.events.vfs_write_page = values[10];
                stat.events.vfs_write_pages = values[11];
                stat.events.vfs_getdents = values[12];
                stat.events.vfs_setattr = values[13];
                stat.events.vfs_flush = values[14];
                stat.events.vfs_fsync = values[15];
                stat.events.vfs_lock = values[16];
                stat.events.vfs_file_release = values[17];
                stat.events.congestion_wait = values[18];
                stat.events.truncation = values[19];
                stat.events.write_extension = values[20];
                stat.events.silly_rename = values[21];
                stat.events.short_read = values[22];
                stat.events.short_write = values[23];
                stat.events.jukebox_delay = values[24];
                stat.events.pnfs_read = values[25];
                stat.events.pnfs_write = values[26];
            }
            "xprt:" => {
                let Some(protocol) = fields.next() else {
                    return Err(Error::Malformed("NFS stats"));
                };

                let size = match (stat.version, protocol) {
                    ("1.0", "tcp") => 10,
                    ("1.0", "udp") => 7,
                    ("1.1", "tcp") => 13,
                    ("1.1", "udp") => 10,
                    // Kernel version <= 4.2 MinLen
                    // See: https://elixir.bootlin.com/linux/v4.2.8/source/net/sunrpc/xprtrdma/xprt_rdma.h#L331
                    ("1.1", "rdma") => 20,
                    _ => return Err(Error::Malformed("NFS stats")),
                };

                let mut values = fields
                    .take(size)
                    .map(|field| field.parse::<u64>())
                    .collect::<Result<Vec<_>, _>>()?;
                if values.len() != size {
                    return Err(Error::Malformed("NFS stats"));
                }

                // Allocate enough for v1.1 stats since zero value for v1.1 stats will be okay
                // in a v1.0 response. Since the stat length is bigger for TCP stats, we use
                // the TCP length here.
                //
                // Note: slice length must be set to length of v1.1 stats to avoid a panic when
                // only v1.0 stats are present.
                // See: https://github.com/prometheus/node_exporter/issues/571.
                //
                // Note: NFS Over RDMA slice length is fieldTransport11RDMAMaxLen
                values.resize(28 + 3, 0);

                // The fields differ depending on the transport protocol (TCP or UDP)
                // From https://utcc.utoronto.ca/%7Ecks/space/blog/linux/NFSMountstatsXprt
                //
                // For the udp RPC transport there is no connection count, connect idle
                // time, or idle time (fields #3, #4, and #5); all other fields are the
                // same. So we set them to 0 here.
                match protocol {
                    "udp" => {
                        values.copy_within(2..size, 5);
                        values[2] = 0;
                        values[3] = 0;
                        values[4] = 0;
                    }
                    "tcp" => {
                        // values.copy_within(13..size, size + 3)
                    }
                    "rdma" => {
                        // 0~10 + 0 + 0 + 0 + 10~
                        values.copy_within(10..size, 13);
                        values[10] = 0;
                        values[11] = 0;
                        values[12] = 0;
                    }
                    _ => {}
                }

                stat.transports.push(TransportStats {
                    protocol,
                    // port: values[0],
                    bind: values[1],
                    connect: values[2],
                    connect_idle_time: values[3],
                    idle_time_seconds: values[4],
                    sends: values[5],
                    receives: values[6],
                    bad_transaction_ids: values[7],
                    cumulative_active_requests: values[8],
                    cumulative_backlog: values[9],

                    // NFS xprt over tcp or udp and statVersion 1.1
                    maximum_rpc_slots_used: values[10],
                    cumulative_sending_queue: values[11],
                    cumulative_pending_queue: values[12],
                    // NFS xprt over rdma and stat version 1.1
                });
            }
            "per-op" => {
                break;
            }
            _ => {}
        }
    }

    // NFS per-operation stats appear last before the next device entry
    stat.operations = parse_nfs_operation_stats(lines)?;

    Ok(stat)
}

fn parse_nfs_operation_stats<'a, I: Iterator<Item = &'a str>>(
    lines: I,
) -> Result<Vec<OperationStats<'a>>, Error> {
    let mut stats = Vec::new();

    for line in lines {
        let mut fields = line.split_ascii_whitespace();
        let Some(first) = fields.next() else { break };
        let operation = first.strip_suffix(':').unwrap_or(first);

        let values = fields
            .take(9)
            .map(|field| field.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() < 8 {
            return Err(Error::Malformed("NFS operation stats"));
        }

        let errors = values.get(8).copied().unwrap_or_default();

        stats.push(OperationStats {
            operation,
            requests: values[0],
            transmissions: values[1],
            major_timeouts: values[2],
            bytes_sent: values[3],
            bytes_received: values[4],
            cumulative_queue_milliseconds: values[5],
            cumulative_total_response_milliseconds: values[6],
            cumulative_total_request_milliseconds: values[7],
            errors,
        });
    }

    Ok(stats)
}

fn parse_mount_info(content: &str) -> Result<Vec<MountInfo<'_>>, Error> {
    let mut infos = Vec::new();

    for line in content.lines() {
        let Some(super_options) = line.split_ascii_whitespace().nth(10) else {
            continue;
        };

        infos.push(MountInfo { super_options })
    }

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_ops() {
        let content = r#"device fs.example.com:/volume4/apps/home-automation/node-red-data mounted on /var/lib/kubelet/pods/1c2215a7-0d92-4df5-83ce-a807bcc2f8c8/volumes/kubernetes.io~nfs/home-automation--node-red-data--pv0001 with fstype nfs4 statvers=1.1
	opts:   rw,vers=4.1,rsize=131072,wsize=131072,namlen=255,acregmin=3,acregmax=60,acdirmin=30,acdirmax=60,hard,proto=tcp,timeo=600,retrans=2,sec=sys,clientaddr=192.168.1.191,local_lock=none
	age:    83520
	impl_id:        name='',domain='',date='0,0'
	caps:   caps=0x3fff7,wtmult=512,dtsize=32768,bsize=0,namlen=255
	nfsv4:  bm0=0xfdffafff,bm1=0xf9be3e,bm2=0x800,acl=0x0,sessions,pnfs=not configured,lease_time=90,lease_expired=0
	sec:    flavor=1,pseudoflavor=1
	events: 52472 472680 16671 57552 2104 9565 749555 9568641 168 24103 1 267134 3350 20097 116581 18214 43757 111141 0 28 9563845 34 0 0 0 0 0 
	bytes:  2021340783 39056395530 0 0 1788561151 39087991255 442605 9557343 
	RPC iostats version: 1.1  p/v: 100003/4 (nfs)
	xprt:   tcp 940 0 2 0 1 938505 938504 0 12756069 0 32 254729 10823602
	per-op statistics
			NULL: 1 1 0 44 24 0 0 0 0
			READ: 34096 34096 0 7103096 1792122744 2272 464840 467945 0
			WRITE: 322308 322308 0 39161277084 56725504 401718334 10139998 411864389 0
			COMMIT: 12541 12541 0 2709896 1304264 342 7179 7819 0
			OPEN: 12637 12637 0 3923256 4659940 871 57185 58251 394
	OPEN_CONFIRM: 0 0 0 0 0 0 0 0 0
		OPEN_NOATTR: 98741 98741 0 25656212 31630800 3366 77710 82693 0
	OPEN_DOWNGRADE: 0 0 0 0 0 0 0 0 0
			CLOSE: 87075 87075 0 18778608 15308496 2026 49131 52399 116
			SETATTR: 24576 24576 0 5825876 6522260 643 34384 35650 0
			FSINFO: 1 1 0 168 152 0 0 0 0
			RENEW: 0 0 0 0 0 0 0 0 0
		SETCLIENTID: 0 0 0 0 0 0 0 0 0
	SETCLIENTID_CONFIRM: 0 0 0 0 0 0 0 0 0
			LOCK: 22512 22512 0 5417628 2521312 1088 17407 18794 2
			LOCKT: 0 0 0 0 0 0 0 0 0
			LOCKU: 21247 21247 0 4589352 2379664 315 8409 9003 0
			ACCESS: 1466 1466 0 298160 246288 22 1394 1492 0
			GETATTR: 52480 52480 0 10015464 12694076 2930 30069 34502 0
			LOOKUP: 11727 11727 0 2518200 2886376 272 16935 17662 3546
		LOOKUP_ROOT: 0 0 0 0 0 0 0 0 0
			REMOVE: 833 833 0 172236 95268 15 4566 4617 68
			RENAME: 11431 11431 0 3150708 1737512 211 52649 53091 0
			LINK: 1 1 0 288 292 0 0 0 0
			SYMLINK: 0 0 0 0 0 0 0 0 0
			CREATE: 77 77 0 18292 23496 0 363 371 11
		PATHCONF: 1 1 0 164 116 0 0 0 0
			STATFS: 7420 7420 0 1394960 1187200 144 4672 4975 0
		READLINK: 4 4 0 704 488 0 1 1 0
			READDIR: 1353 1353 0 304024 2902928 11 4326 4411 0
		SERVER_CAPS: 9 9 0 1548 1476 0 3 3 0
		DELEGRETURN: 232 232 0 48896 37120 811 300 1115 0
			GETACL: 0 0 0 0 0 0 0 0 0
			SETACL: 0 0 0 0 0 0 0 0 0
	FS_LOCATIONS: 0 0 0 0 0 0 0 0 0
	RELEASE_LOCKOWNER: 0 0 0 0 0 0 0 0 0
			SECINFO: 0 0 0 0 0 0 0 0 0
	FSID_PRESENT: 0 0 0 0 0 0 0 0 0
		EXCHANGE_ID: 2 2 0 464 200 0 0 0 0
	CREATE_SESSION: 1 1 0 192 124 0 0 0 0
	DESTROY_SESSION: 0 0 0 0 0 0 0 0 0
		SEQUENCE: 0 0 0 0 0 0 0 0 0
	GET_LEASE_TIME: 0 0 0 0 0 0 0 0 0
	RECLAIM_COMPLETE: 1 1 0 124 88 0 81 81 0
		LAYOUTGET: 0 0 0 0 0 0 0 0 0
	GETDEVICEINFO: 0 0 0 0 0 0 0 0 0
	LAYOUTCOMMIT: 0 0 0 0 0 0 0 0 0
	LAYOUTRETURN: 0 0 0 0 0 0 0 0 0
	SECINFO_NO_NAME: 0 0 0 0 0 0 0 0 0
	TEST_STATEID: 0 0 0 0 0 0 0 0 0
	FREE_STATEID: 10413 10413 0 1416168 916344 147 3518 3871 10413
	GETDEVICELIST: 0 0 0 0 0 0 0 0 0
	BIND_CONN_TO_SESSION: 0 0 0 0 0 0 0 0 0
	DESTROY_CLIENTID: 0 0 0 0 0 0 0 0 0
			SEEK: 0 0 0 0 0 0 0 0 0
		ALLOCATE: 0 0 0 0 0 0 0 0 0
		DEALLOCATE: 0 0 0 0 0 0 0 0 0
		LAYOUTSTATS: 0 0 0 0 0 0 0 0 0
			CLONE: 0 0 0 0 0 0 0 0 0
			COPY: 0 0 0 0 0 0 0 0 0
	OFFLOAD_CANCEL: 0 0 0 0 0 0 0 0 0
			LOOKUPP: 0 0 0 0 0 0 0 0 0
		LAYOUTERROR: 0 0 0 0 0 0 0 0 0"#;

        let _mounts = parse_mount_stats(content).unwrap();
    }

    #[test]
    fn extended_rdma() {
        let content = r#"device <nfsserver>:<nfsmount> mounted on <mountpoint> with fstype nfs statvers=1.1
        opts:   ro,vers=3,rsize=1048576,wsize=1048576,namlen=255,acregmin=120,acregmax=120,acdirmin=120,acdirmax=120,hard,nocto,forcerdirplus,proto=rdma,nconnect=16,port=20049,timeo=600,retrans=2,sec=sys,mountaddr=172.16.40.20,mountvers=3,mountport=0,mountproto=tcp,local_lock=none
        age:    1270876
        caps:   caps=0xf,wtmult=4096,dtsize=131072,bsize=0,namlen=255
        sec:    flavor=1,pseudoflavor=1
        events: 512052 36601115 0 68 1498583 16514 38815015 0 41584 2654459933 0 0 0 0 1527715 0 0 1498575 0 0 0 0 0 0 0 0 0
        bytes:  3104202770327296 0 0 0 2013200952170479 0 491504202537 0
        RPC iostats version: 1.1  p/v: 100003/3 (nfs)
        xprt:   rdma 0 0 5808 62 0 494490723 494490687 36 10032963746 1282789 107150285 1226637531 2673889 135120843409861 135119397156505 266368832 75716996 0 7853 0 0 0 0 119328 1336431717 0 96
        xprt:   rdma 0 0 14094 145 0 492392334 492392307 27 7078693624 2509627 105561370 1280878332 2659446 142218924010291 142217463504063 276368040 94761838 0 7610 0 0 0 0 207977 1389069860 0 103
        xprt:   rdma 0 0 16107 156 0 522755125 522755092 33 9119562599 1147699 109077860 1491898147 2566003 167152062826463 167149287506014 284931680 83011025 0 6229 0 0 0 0 221408 1603518232 0 82
        xprt:   rdma 0 0 7808 82 0 441542046 441542010 36 7226132207 2519174 111096004 955223347 2676765 105741904708009 105740125663595 275613584 80373159 0 8893 0 0 0 0 149479 1068962768 0 76
        xprt:   rdma 0 0 15018 167 0 508091827 508091764 63 19817677255 36702583 108265928 1258185459 2438516 138247436686102 138246196289594 270162080 74962306 0 13328 0 0 0 0 268433 1368837472 0 66
        xprt:   rdma 0 0 14321 149 0 530246310 530246275 35 9723190432 2392024 111099700 1494204555 2589805 166691166581904 166689567426908 289995492 85067377 0 8010 0 0 0 0 214511 1607864447 0 100
        xprt:   rdma 0 0 7863 84 0 459019689 459019642 47 11809253102 1716688 111825219 1032758664 2564226 114416685286438 114414936423706 290494252 73702102 0 6927 0 0 0 0 134453 1147121864 0 79
        xprt:   rdma 0 0 7702 84 3 497598986 497598931 55 11816221496 3924722 106922130 1382063307 2506108 153967067193941 153965665472218 286222584 84094006 0 5875 0 0 0 0 127347 1491469045 0 66
        xprt:   rdma 0 0 18341 202 0 477721151 477721073 78 15204400959 40562626 106645745 1291616653 3091375 144533696686651 144529688231163 278135800 73821525 0 6795 0 0 0 0 251097 1401327563 0 64
        xprt:   rdma 0 0 8228 90 4 453155092 453155063 29 7884786894 1591225 112197590 1026006338 2742688 114591819605673 114590175821191 275541944 85857259 0 7487 0 0 0 0 143044 1140917892 0 76
        xprt:   rdma 0 0 7843 83 0 446480377 446480324 53 12267986428 2958997 111971246 963162784 2693433 107176282309753 107174637802555 290269096 101100410 0 7825 0 0 0 0 141735 1077797328 0 83
        xprt:   rdma 0 0 7582 86 0 423315608 423315567 41 10197484604 2076993 109207538 785978455 2650354 86090211449474 86088475571312 279912524 87676008 0 7491 0 0 0 0 137533 897807641 0 101
        xprt:   rdma 0 0 7767 84 0 482538465 482538424 41 8935200479 1344778 112200583 1192341640 2644896 132860698423762 132858881459050 273354060 75337030 0 5941 0 0 0 0 127842 1307164736 0 97
        xprt:   rdma 0 0 14526 148 2 537745063 537745007 56 20756072620 3970332320 109539564 1363647371 2503250 148793734936250 148791264145401 291888720 90344151 0 7471 0 0 0 0 211057 1475661285 0 82
        xprt:   rdma 0 0 14300 151 0 495357347 495357316 31 8703101643 1451809 112315311 1303804607 2620502 145680743007170 145678880292235 288046696 98018259 0 7241 0 0 0 0 209396 1418712657 0 139
        xprt:   rdma 0 0 7700 82 0 466611083 466611050 33 8540498291 4082864 114740300 1059770596 2523155 117376668239921 117375375683167 260927576 78437075 0 6691 0 0 0 0 130878 1177008175 1 76
        per-op statistics
                NULL: 16 16 0 640 384 320 11 331 0
             GETATTR: 512052 512052 0 79823516 57349824 107131 612667 751847 0
             SETATTR: 0 0 0 0 0 0 0 0 0
              LOOKUP: 16713 16713 0 3040536 3706344 560 17488 20232 346
              ACCESS: 211705 211705 0 33860920 25404600 37059 229754 283822 0
            READLINK: 0 0 0 0 0 0 0 0 0
                READ: 2654501510 2654501510 0 445911966900 2013540728551504 6347457114 31407021389 37927280438 0
               WRITE: 0 0 0 0 0 0 0 0 0
              CREATE: 0 0 0 0 0 0 0 0 0
               MKDIR: 0 0 0 0 0 0 0 0 0
             SYMLINK: 0 0 0 0 0 0 0 0 0
               MKNOD: 0 0 0 0 0 0 0 0 0
              REMOVE: 0 0 0 0 0 0 0 0 0
               RMDIR: 0 0 0 0 0 0 0 0 0
              RENAME: 0 0 0 0 0 0 0 0 0
                LINK: 0 0 0 0 0 0 0 0 0
             READDIR: 0 0 0 0 0 0 0 0 0
         READDIRPLUS: 0 0 0 0 0 0 0 0 0
              FSSTAT: 56356 56356 0 6243572 9467808 82068 74356 159001 0
              FSINFO: 2 2 0 184 328 0 0 0 0
            PATHCONF: 1 1 0 92 140 0 0 0 0
              COMMIT: 0 0 0 0 0 0 0 0 0"#;

        let _mounts = parse_mount_stats(content).unwrap();
    }

    #[test]
    fn nfs_over_rdma() {
        let content = r#"device <nfsserver>:<nfsmount> mounted on <mountpoint> with fstype nfs statvers=1.1
        opts:   ro,vers=3,rsize=1048576,wsize=1048576,namlen=255,acregmin=120,acregmax=120,acdirmin=120,acdirmax=120,hard,nocto,forcerdirplus,proto=rdma,nconnect=16,port=20049,timeo=600,retrans=2,sec=sys,mountaddr=172.16.40.20,mountvers=3,mountport=0,mountproto=tcp,local_lock=none
        age:    1270876
        caps:   caps=0xf,wtmult=4096,dtsize=131072,bsize=0,namlen=255
        sec:    flavor=1,pseudoflavor=1
        events: 512052 36601115 0 68 1498583 16514 38815015 0 41584 2654459933 0 0 0 0 1527715 0 0 1498575 0 0 0 0 0 0 0 0 0
        bytes:  3104202770327296 0 0 0 2013200952170479 0 491504202537 0
        RPC iostats version: 1.1  p/v: 100003/3 (nfs)
        xprt:   rdma 0 0 5808 62 0 494490723 494490687 36 10032963746 1282789 107150285 1226637531 2673889 135120843409861 135119397156505 266368832 75716996 0 7853 0 0 0 0 119328 1336431717 0 96
        per-op statistics
                NULL: 16 16 0 640 384 320 11 331 0"#;
        let want = vec![Mount {
            device: "<nfsserver>:<nfsmount>",
            mount: "<mountpoint>",
            typ: "nfs",
            stats: MountStats {
                version: "1.1",
                // options: map[string]string{"acdirmax": "120", "acdirmin": "120", "acregmax": "120",
                //     "acregmin": "120", "forcerdirplus": "", "hard": "", "local_lock": "none",
                //     "mountaddr": "172.16.40.20", "mountport": "0", "mountproto": "tcp", "mountvers": "3",
                //     "namlen": "255", "nconnect": "16", "nocto": "", "port": "20049", "proto": "rdma",
                //     "retrans": "2", "ro": "", "rsize": "1048576", "sec": "sys", "timeo": "600",
                //     "vers": "3", "wsize": "1048576"},
                opts: "ro,vers=3,rsize=1048576,wsize=1048576,namlen=255,acregmin=120,acregmax=120,acdirmin=120,acdirmax=120,hard,nocto,forcerdirplus,proto=rdma,nconnect=16,port=20049,timeo=600,retrans=2,sec=sys,mountaddr=172.16.40.20,mountvers=3,mountport=0,mountproto=tcp,local_lock=none",
                age: Duration::from_secs(1270876),
                bytes: BytesStats {
                    read: 3104202770327296,
                    write: 0,
                    direct_read: 0,
                    direct_write: 0,
                    read_total: 2013200952170479,
                    write_total: 0,
                    read_pages: 491504202537,
                    write_pages: 0,
                },
                events: EventsStats {
                    inode_revalidate: 512052,
                    dnode_revalidate: 36601115,
                    data_invalidate: 0,
                    attribute_invalidate: 68,
                    vfs_open: 1498583,
                    vfs_lookup: 16514,
                    vfs_access: 38815015,
                    vfs_update_page: 0,
                    vfs_read_page: 41584,
                    vfs_read_pages: 2654459933,
                    vfs_write_page: 0,
                    vfs_write_pages: 0,
                    vfs_getdents: 0,
                    vfs_setattr: 0,
                    vfs_flush: 1527715,
                    vfs_fsync: 0,
                    vfs_lock: 0,
                    vfs_file_release: 1498575,
                    congestion_wait: 0,
                    truncation: 0,
                    write_extension: 0,
                    silly_rename: 0,
                    short_read: 0,
                    short_write: 0,
                    jukebox_delay: 0,
                    pnfs_read: 0,
                    pnfs_write: 0,
                },
                operations: vec![OperationStats {
                    operation: "NULL",
                    requests: 16,
                    transmissions: 16,
                    major_timeouts: 0,
                    bytes_sent: 640,
                    bytes_received: 384,
                    cumulative_queue_milliseconds: 320,
                    cumulative_total_response_milliseconds: 11,
                    cumulative_total_request_milliseconds: 331,
                    errors: 0,
                }],
                transports: vec![TransportStats {
                    protocol: "rdma",
                    // port: 0,
                    bind: 0,
                    connect: 5808,
                    connect_idle_time: 62,
                    idle_time_seconds: 0,
                    sends: 494490723,
                    receives: 494490687,
                    bad_transaction_ids: 36,
                    cumulative_active_requests: 10032963746,
                    cumulative_backlog: 1282789,
                    maximum_rpc_slots_used: 0,
                    cumulative_sending_queue: 0,
                    cumulative_pending_queue: 0,
                    // ReadChunkCount: 107150285,
                    // WriteChunkCount: 1226637531,
                    // ReplyChunkCount: 2673889,
                    // TotalRdmaRequest: 135120843409861,
                    // PullupCopyCount: 135119397156505,
                    // HardwayRegisterCount: 266368832,
                    // FailedMarshalCount: 75716996,
                    // BadReplyCount: 0,
                    // MrsRecovered: 7853,
                    // MrsOrphaned: 0,
                    // MrsAllocated: 0,
                    // EmptySendctxQ: 0,
                    // TotalRdmaReply: 0,
                    // FixupCopyCount: 119328,
                    // ReplyWaitsForSend: 1336431717,
                    // LocalInvNeeded: 0,
                    // NomsgCallCount: 96,
                    // BcallCount: 0,
                }],
            },
        }];

        let mounts = parse_mount_stats(content).unwrap();
        assert_eq!(want, mounts);
    }

    #[test]
    fn multi_devices() {
        let content = r#"device rootfs mounted on / with fstype rootfs
device sysfs mounted on /sys with fstype sysfs
device proc mounted on /proc with fstype proc
device /dev/sda1 mounted on / with fstype ext4
device 192.168.1.1:/srv/test mounted on /mnt/nfs/test with fstype nfs4 statvers=1.1
	opts:	rw,vers=4.0,rsize=1048576,wsize=1048576,namlen=255,acregmin=3,acregmax=60,acdirmin=30,acdirmax=60,hard,proto=tcp,port=0,timeo=600,retrans=2,sec=sys,mountaddr=192.168.1.1,clientaddr=192.168.1.5,local_lock=none
	age:	13968
	caps:	caps=0xfff7,wtmult=512,dtsize=32768,bsize=0,namlen=255
	nfsv4:	bm0=0xfdffafff,bm1=0xf9be3e,bm2=0x0,acl=0x0,pnfs=not configured
	sec:	flavor=1,pseudoflavor=1
	events:	52 226 0 0 1 13 398 0 0 331 0 47 0 0 77 0 0 77 0 0 0 0 0 0 0 0 0
	bytes:	1207640230 0 0 0 1210214218 0 295483 0
	RPC iostats version: 1.0  p/v: 100003/4 (nfs)
	xprt:	tcp 832 0 1 0 11 6428 6428 0 12154 0 24 26 5726
	per-op statistics
	        NULL: 0 0 0 0 0 0 0 0
	        READ: 1298 1298 0 207680 1210292152 6 79386 79407
	       WRITE: 0 0 0 0 0 0 0 0
	      ACCESS: 2927395007 2927394995 0 526931094212 362996810236 18446743919241604546 1667369447 1953587717
"#;
        let want = vec![
            Mount {
                device: "rootfs",
                mount: "/",
                typ: "rootfs",
                stats: Default::default(),
            },
            Mount {
                device: "sysfs",
                mount: "/sys",
                typ: "sysfs",
                stats: Default::default(),
            },
            Mount {
                device: "proc",
                mount: "/proc",
                typ: "proc",
                stats: Default::default(),
            },
            Mount {
                device: "/dev/sda1",
                mount: "/",
                typ: "ext4",
                stats: Default::default(),
            },
            Mount {
                device: "192.168.1.1:/srv/test",
                mount: "/mnt/nfs/test",
                typ: "nfs4",
                stats: MountStats {
                    version: "1.1",
                    // Opts: map[string]string{"rw": "", "vers": "4.0",
                    //     "rsize": "1048576", "wsize": "1048576", "namlen": "255", "acregmin": "3",
                    //     "acregmax": "60", "acdirmin": "30", "acdirmax": "60", "hard": "",
                    //     "proto": "tcp", "port": "0", "timeo": "600", "retrans": "2",
                    //     "sec": "sys", "mountaddr": "192.168.1.1", "clientaddr": "192.168.1.5",
                    //     "local_lock": "none",
                    // },
                    opts: "rw,vers=4.0,rsize=1048576,wsize=1048576,namlen=255,acregmin=3,acregmax=60,acdirmin=30,acdirmax=60,hard,proto=tcp,port=0,timeo=600,retrans=2,sec=sys,mountaddr=192.168.1.1,clientaddr=192.168.1.5,local_lock=none",
                    age: Duration::from_secs(13968),
                    bytes: BytesStats {
                        read: 1207640230,
                        read_total: 1210214218,
                        read_pages: 295483,
                        ..Default::default()
                    },
                    events: EventsStats {
                        inode_revalidate: 52,
                        dnode_revalidate: 226,
                        vfs_open: 1,
                        vfs_lookup: 13,
                        vfs_access: 398,
                        vfs_read_pages: 331,
                        vfs_write_pages: 47,
                        vfs_flush: 77,
                        vfs_file_release: 77,
                        ..Default::default()
                    },
                    operations: vec![
                        OperationStats {
                            operation: "NULL",
                            requests: 0,
                            transmissions: 0,
                            major_timeouts: 0,
                            bytes_sent: 0,
                            bytes_received: 0,
                            cumulative_queue_milliseconds: 0,
                            cumulative_total_response_milliseconds: 0,
                            cumulative_total_request_milliseconds: 0,
                            errors: 0,
                        },
                        OperationStats {
                            operation: "READ",
                            requests: 1298,
                            transmissions: 1298,
                            major_timeouts: 0,
                            bytes_sent: 207680,
                            bytes_received: 1210292152,
                            cumulative_queue_milliseconds: 6,
                            cumulative_total_response_milliseconds: 79386,
                            cumulative_total_request_milliseconds: 79407,

                            errors: 0,
                        },
                        OperationStats {
                            operation: "WRITE",

                            requests: 0,
                            transmissions: 0,
                            major_timeouts: 0,
                            bytes_sent: 0,
                            bytes_received: 0,
                            cumulative_queue_milliseconds: 0,
                            cumulative_total_response_milliseconds: 0,
                            cumulative_total_request_milliseconds: 0,
                            errors: 0,
                        },
                        OperationStats {
                            operation: "ACCESS",
                            requests: 2927395007,
                            transmissions: 2927394995,
                            major_timeouts: 0,
                            bytes_sent: 526931094212,
                            bytes_received: 362996810236,
                            cumulative_queue_milliseconds: 18446743919241604546,
                            cumulative_total_response_milliseconds: 1667369447,
                            cumulative_total_request_milliseconds: 1953587717,

                            errors: 0,
                        },
                    ],
                    transports: vec![TransportStats {
                        protocol: "tcp",
                        // Port: 832,
                        bind: 0,
                        connect: 1,
                        connect_idle_time: 0,
                        idle_time_seconds: 11,
                        sends: 6428,
                        receives: 6428,
                        bad_transaction_ids: 0,
                        cumulative_active_requests: 12154,
                        cumulative_backlog: 0,
                        maximum_rpc_slots_used: 24,
                        cumulative_sending_queue: 26,
                        cumulative_pending_queue: 5726,
                    }],
                },
            },
        ];

        let mounts = parse_mount_stats(content).unwrap();
        assert_eq!(want, mounts);
    }

    #[test]
    fn nfs3_over_tcp_with_stats_10() {
        let content = r#"device 192.168.1.1:/srv mounted on /mnt/nfs with fstype nfs statvers=1.0
xprt: tcp 1 2 3 4 5 6 7 8 9 10"#;
        let mounts = parse_mount_stats(content).unwrap();
        let want = vec![Mount {
            device: "192.168.1.1:/srv",
            mount: "/mnt/nfs",
            typ: "nfs",
            stats: MountStats {
                version: "1.0",
                opts: "",
                age: Default::default(),
                bytes: Default::default(),
                events: Default::default(),
                operations: vec![],
                transports: vec![TransportStats {
                    protocol: "tcp",
                    // port:                     1,
                    bind: 2,
                    connect: 3,
                    connect_idle_time: 4,
                    idle_time_seconds: 5,
                    sends: 6,
                    receives: 7,
                    bad_transaction_ids: 8,
                    cumulative_active_requests: 9,
                    cumulative_backlog: 10,
                    maximum_rpc_slots_used: 0,   // these three are not
                    cumulative_sending_queue: 0, // present in statvers=1.0
                    cumulative_pending_queue: 0, //
                }],
            },
        }];

        assert_eq!(want, mounts)
    }

    #[test]
    fn nfs3_over_udp_with_stats_10() {
        let content = r#"device 192.168.1.1:/srv mounted on /mnt/nfs with fstype nfs statvers=1.0
xprt: udp 1 2 3 4 5 6 7"#;
        let mounts = parse_mount_stats(content).unwrap();
        let want = vec![Mount {
            device: "192.168.1.1:/srv",
            mount: "/mnt/nfs",
            typ: "nfs",
            stats: MountStats {
                version: "1.0",
                opts: "",
                age: Default::default(),
                bytes: Default::default(),
                events: Default::default(),
                operations: vec![],
                transports: vec![TransportStats {
                    protocol: "udp",
                    // Port: 1,
                    bind: 2,
                    connect: 0,
                    connect_idle_time: 0,
                    idle_time_seconds: 0,
                    sends: 3,
                    receives: 4,
                    bad_transaction_ids: 5,
                    cumulative_active_requests: 6,
                    cumulative_backlog: 7,
                    maximum_rpc_slots_used: 0,   // these three are not
                    cumulative_sending_queue: 0, // present in statvers=1.0
                    cumulative_pending_queue: 0, //
                }],
            },
        }];

        assert_eq!(want, mounts);
    }

    #[test]
    fn nfs3_over_tcp_with_stats_11() {
        let content = r#"device 192.168.1.1:/srv mounted on /mnt/nfs with fstype nfs statvers=1.1
xprt: tcp 1 2 3 4 5 6 7 8 9 10 11 12 13"#;

        let mounts = parse_mount_stats(content).unwrap();
        let want = vec![Mount {
            device: "192.168.1.1:/srv",
            mount: "/mnt/nfs",
            typ: "nfs",
            stats: MountStats {
                version: "1.1",
                opts: "",
                age: Default::default(),
                bytes: Default::default(),
                events: Default::default(),
                operations: vec![],
                transports: vec![TransportStats {
                    protocol: "tcp",
                    // Port: 1,
                    bind: 2,
                    connect: 3,
                    connect_idle_time: 4,
                    idle_time_seconds: 5,
                    sends: 6,
                    receives: 7,
                    bad_transaction_ids: 8,
                    cumulative_active_requests: 9,
                    cumulative_backlog: 10,
                    maximum_rpc_slots_used: 11,
                    cumulative_sending_queue: 12,
                    cumulative_pending_queue: 13,
                }],
            },
        }];

        assert_eq!(want, mounts);
    }

    #[test]
    fn nfs3_over_udp_with_stats_11() {
        let content = r#"device 192.168.1.1:/srv mounted on /mnt/nfs with fstype nfs statvers=1.1
xprt: udp 1 2 3 4 5 6 7 8 9 10"#;
        let mounts = parse_mount_stats(content).unwrap();
        let want = vec![Mount {
            device: "192.168.1.1:/srv",
            mount: "/mnt/nfs",
            typ: "nfs",
            stats: MountStats {
                version: "1.1",
                opts: "",
                age: Default::default(),
                bytes: Default::default(),
                events: Default::default(),
                operations: vec![],
                transports: vec![TransportStats {
                    protocol: "udp",
                    // port: 1,
                    bind: 2,
                    connect: 0,           // these three are not
                    connect_idle_time: 0, // present for UDP
                    idle_time_seconds: 0, //
                    sends: 3,
                    receives: 4,
                    bad_transaction_ids: 5,
                    cumulative_active_requests: 6,
                    cumulative_backlog: 7,
                    maximum_rpc_slots_used: 8,
                    cumulative_sending_queue: 9,
                    cumulative_pending_queue: 10,
                }],
            },
        }];

        assert_eq!(want, mounts);
    }

    #[test]
    fn ok() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/self/mountstats").unwrap();
        let _mounts = parse_mount_stats(&content).unwrap();
    }
}
