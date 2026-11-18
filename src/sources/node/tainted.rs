use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

/*
Lists all known kernel taint flags in bit-position order.
Reference: https://www.kernel.org/doc/html/latest/admin-guide/tainted-kernels.html

P: Proprietary module was loaded.
F: Module was force loaded.
S: SMP kernel oops on an officially SMP incapable processor.
R: Module was force unloaded.
M: Processor reported a Machine Check Exception (MCE).
B: Bad page referenced or some unexpected page flags.
U: Taint requested by userspace application.
D: Kernel died recently, i.e. there was an OOPS or BUG.
A: An ACPI table was overridden by user.
W: Kernel issued warning.
C: Staging driver was loaded.
I: Workaround for bug in platform firmware applied.
O: Externally-built (out-of-tree) module was loaded.
E: Unsigned module was loaded.
L: Soft lockup occurred.
K: Kernel has been live patched.
X: Auxiliary taint, defined and used by distros.
T: The kernel was built with the struct randomization plugin.
N: An in-kernel test such as a KUnit test has been run.
J: Userspace used a mutating debug operation in fwctl.
*/

const TAINT_BITS: [&str; 20] = [
    "P", "F", "S", "R", "M", "B", "U", "D", "A", "W", "C", "I", "O", "E", "L", "K", "X", "T", "N",
    "J",
];

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let path = paths.proc().join("sys/kernel/tainted");
    let value = read_file_no_stat(path)?.trim().parse::<u64>()?;

    let mut metrics = Vec::with_capacity(20);
    for (offset, flag) in TAINT_BITS.iter().enumerate() {
        let set = value & (1 << offset) != 0;

        metrics.push(Metric::gauge_with_tags(
            "node_kernel_tainted",
            "Taint flags set on the running Linux kernel, as reported by /proc/sys/kernel/tainted. Value is 1 if the flag is set, 0 otherwise. See https://www.kernel.org/doc/html/latest/admin-guide/tainted-kernels.html for flag meanings.",
            set,
            tags!(
                "flag" => *flag,
                "bit" => offset,
            )
        ))
    }

    Ok(metrics)
}
