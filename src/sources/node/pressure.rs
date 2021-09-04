/// The PSI / pressure interface is described at
///   https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/Documentation/accounting/psi.txt
/// Each resource (cpu, io, memory, ...) is exposed as a single file.
/// Each file may contain up to two lines, one for "some" pressure and one for "full" pressure.
/// Each line contains several averages (over n seconds) and a total in µs.
///
/// Example io pressure file:
/// > some avg10=0.06 avg60=0.21 avg300=0.99 total=8537362
/// > full avg10=0.00 avg60=0.13 avg300=0.96 total=8183134

pub async fn gather() {}