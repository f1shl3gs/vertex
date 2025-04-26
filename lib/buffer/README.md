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
