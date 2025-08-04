# Tail

## Multiline


## Rotate

| Strategy          |  Support  |  Loss data  |
|-------------------|:---------:|:-----------:|
| Rename and create |  &check;  |             |
| Copy Truncate     |  &cross;  |             |

## Backpressure

If the `Output` is too slow, we might loss some data if the file rotate. `Ratelimit` can be implemented
in the Output.

## Performance
Be aware, the bench `Output` is barely do nothing, so we don't have any backpressure issue,
but in real world, this cannot be ignored.

#### Simple Line Throughput
Our log generator is awful, it can reach only around 40~50 M/s, so i test the `tail` with
`cat from.log >> to.log` to avoid the bad writer performance.

```text
consumed:   2097151834, rate:        1999.9998 M/s
consumed:   2097152126, rate:           0.0003 M/s
```

#### JSON
TODO
