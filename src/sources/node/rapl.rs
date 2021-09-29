/// Expose various statistics from /sys/class/powercap
///
/// http://web.eece.maine.edu/~vweaver/projects/rapl/
/// ## RAPL(Running Average Power Limit) on Linux
///
/// There are currently *three* ways to read RAPL results using the Linux kernel:
/// - Reading the files under /sys/class/powercap/intel-rapl/intel-rapl:0 using the powercap interface. This requires no special permissions, and was introduced in Linux 3.13
/// - Using the perf_event interface with Linux 3.14 or newer. This requires root or a paranoid less than 1 (as do all system wide measurements with -a) sudo perf stat -a -e "power/energy-cores/" /bin/ls Available events can be found via perf list or under /sys/bus/event_source/devices/power/events/
/// - Using raw-access to the underlying MSRs under /dev/msr. This requires root.
/// Not that you cannot get readings for individual processes, the results are for the entire CPU socket.

use crate::event::Metric;
use crate::sources::node::errors::Error;
use crate::sources::node::read_into;

// RaplZone stores the information for one RAPL power zone
struct RaplZone {
    // name of RAPL zone from file "name"
    name: String,

    // index (different value for duplicate names)
    index: i32,

    // filesystem path of RaplZone
    path: String,

    // max RAPL microjoule value
    max_microjoules: u64,
}

impl RaplZone {
    /// `get_energy_microjoules` returns the current microjoule value from the zone energy counter
    /// https://www.kernel.org/doc/Documentation/power/powercap/powercap.txt
    async fn get_energy_microjoules(&self) -> Result<u64, Error> {
        let path = format!("{}/energy_uj", self.path);
        read_into(path).await
    }
}

/// `get_rapl_zones` returns a slice of RaplZones
/// When RAPL files are not present, returns nil with error
/// https://www.kernel.org/doc/Documentation/power/powercap/powercap.txt
async fn get_rapl_zones(sys_path: &str) -> Result<Vec<RaplZone>, ()> {
    todo!()
}

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, Error> {
    todo!()
}
