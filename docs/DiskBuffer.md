# Disk Buffer

## File layout
```text
|               Header(16 + n)                                           |       Record(8 + n)                  | ... | Record |
|  Magic(4) + Version(1) + Padding(3) + Timestamp(8) + First Index(8)    |       Record Head(8)       | Data(n) | ...
|                                                                        | Offset(4) + Data Length(4) |    ...  | ...
```

## Read/Write
```text
| Header |   Record   | 
```