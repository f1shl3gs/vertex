# Buffer 

## Files
Disk buffer has theos files
```text
ledger.state
0000.chunk
0001.chunk
....
buffer.lock
```

## Chunk Layout
The `chunk` file size is fixed, and less than 256M.

### Record Layout
```text
|-- Length --|-- CRC32 --|-- ID --|-- payload --| ... |-- Length --|-- CRC32 --|-- ID --|-- payload --|
|   32 bit   |  32 bit   | 64 bit |  n bytes    | ... |   32 bit   |  32 bit   | 64 bit |  n bytes    |
```

## Why not Tokio's File!?
Tokio's file operations is not real async operations, it spawns a blocking thread (if none), and call the
syscall in it. So, Tokio do not help with the performance, the only benefits is file operations won't
block the current thread. 

- Writer will never block, unless disk is full

- Where we will block? Writer will not unless the disk if full, Reader will not too, cause if there is 
no data, `BufReader::fill_buf` will return a empty slice then wait on Notify.

- Block operations is sent to blocking threads, and the poll result is kind of unpredictable,
`assert_pending` or `assert_ready` might fail occasionally.


## TODO
Add a cache to the disk implement, which can avoid deserialization when read records, a simple queue
should be enough, and the size should tracked and limit to avoid memory bloat

## Performance
```text
------------------------------------------- Memory ------------------------------------------
 RECORD_SIZE   BYTES    RECORDS_PER_SEC  BYTES_PER_SEC     TIME  USER_CPU  SYS_CPU  TOTAL_CPU
         128   1.3GB     6220448.66 r/s     759.33 M/s    1.61s   137.47%   48.52%    185.99%
         256   2.6GB     5851396.64 r/s    1428.56 M/s    1.71s   136.92%   46.81%    183.73%
         512   5.1GB     5907619.56 r/s    2884.58 M/s    1.69s   137.65%   46.67%    184.32%
        1024  10.2GB     6144408.52 r/s    6000.40 M/s    1.63s   139.48%   46.08%    185.56%
        2048  20.5GB     5938641.09 r/s   11598.91 M/s    1.68s   138.37%   46.32%    184.69%
        4096    41GB     6236950.84 r/s   24363.09 M/s    1.60s   136.59%   49.90%    186.48%
-------------------------------------------  Disk  ------------------------------------------
 RECORD_SIZE   BYTES    RECORDS_PER_SEC  BYTES_PER_SEC     TIME  USER_CPU  SYS_CPU  TOTAL_CPU
         128   128MB      931664.45 r/s     113.73 M/s    1.07s   127.64%  160.25%    287.88%
         256   256MB      910724.12 r/s     222.34 M/s    1.10s   116.57%  179.41%    295.99%
         512   512MB      782429.28 r/s     382.05 M/s    1.28s   111.10%  178.39%    289.50%
        1024     1GB      622865.94 r/s     608.27 M/s    1.61s    90.94%  189.35%    280.29%
        2048     2GB      454564.02 r/s     887.82 M/s    2.20s    86.82%  193.19%    280.01%
        4096   4.1GB      302904.71 r/s    1183.22 M/s    3.30s    79.06%  193.86%    272.92%
```
