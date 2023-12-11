# Vertex Transformation Language

## Scripting language
```

if .should_handle {
  // we don't want to handle this
  return
}

if .foo == "bar" {
  .foo = "new bar"
}

.bar = .foo
.arr = [.foo, .bar]

.ts = now()

// args are enrichment 
//   1. table name
//   2. path
//   3. column to match
//   4. columns to insert 
get_enrichment("ip_info",  .ip, "ip", ["env", "group"])

```

