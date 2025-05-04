## Benchmark

uses [hyperfine](https://github.com/sharkdp/hyperfine) to conduct benchmark on tascli insertion, list and deletions. `hyperfine` need to be installed separately on the system

`basic.sh` benchmarks task, record insertion, listing and deletion.

`with_config.sh` benchmarks the same but with a configuration file.

As shown, `tascli` has no background process, but it is fast, how fast is it on your machine?

### Example Run

```
$ ./basic.sh
Benchmark 1: Task Insertion
  Time (mean ± σ):       2.0 ms ±   0.2 ms    [User: 1.1 ms, System: 0.7 ms]
  Range (min … max):     1.8 ms …   2.8 ms    50 runs

Benchmark 1: Record Insertion
  Time (mean ± σ):       2.4 ms ±   0.2 ms    [User: 1.2 ms, System: 0.9 ms]
  Range (min … max):     2.2 ms …   3.2 ms    50 runs

Benchmark 1: List Tasks
  Time (mean ± σ):       2.7 ms ±   0.2 ms    [User: 1.4 ms, System: 1.0 ms]
  Range (min … max):     2.6 ms …   4.0 ms    50 runs

Benchmark 1: Task Deletion
  Time (mean ± σ):       3.1 ms ±   0.3 ms    [User: 1.8 ms, System: 1.9 ms]
  Range (min … max):     2.9 ms …   4.6 ms    50 runs

Benchmark 1: Record Deletion
  Time (mean ± σ):       3.1 ms ±   0.2 ms    [User: 1.8 ms, System: 1.9 ms]
  Range (min … max):     2.9 ms …   3.9 ms    50 runs
```
