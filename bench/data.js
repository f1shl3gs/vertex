window.BENCHMARK_DATA = {
  "lastUpdate": 1673709593510,
  "repoUrl": "https://github.com/f1shl3gs/vertex",
  "entries": {
    "prometheus": [
      {
        "commit": {
          "author": {
            "email": "fishlegs.engerman@gmail.com",
            "name": "f1shl3gs",
            "username": "f1shl3gs"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "49dfea83db53aeefe58e59116dfd484c0852e188",
          "message": "Merge pull request #710 from f1shl3gs/fix_bench_charts_name\n\nfix bench name",
          "timestamp": "2023-01-14T22:59:04+08:00",
          "tree_id": "7abdd153c16fac584f3033b1748d455a947b0c3c",
          "url": "https://github.com/f1shl3gs/vertex/commit/49dfea83db53aeefe58e59116dfd484c0852e188"
        },
        "date": 1673708570058,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "prometheus/parse_text",
            "value": 717210,
            "unit": "ns/op"
          }
        ]
      }
    ],
    "metrics": [
      {
        "commit": {
          "author": {
            "email": "fishlegs.engerman@gmail.com",
            "name": "f1shl3gs",
            "username": "f1shl3gs"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "49dfea83db53aeefe58e59116dfd484c0852e188",
          "message": "Merge pull request #710 from f1shl3gs/fix_bench_charts_name\n\nfix bench name",
          "timestamp": "2023-01-14T22:59:04+08:00",
          "tree_id": "7abdd153c16fac584f3033b1748d455a947b0c3c",
          "url": "https://github.com/f1shl3gs/vertex/commit/49dfea83db53aeefe58e59116dfd484c0852e188"
        },
        "date": 1673708579177,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "metrics/without_labels",
            "value": 90.064,
            "unit": "ns/op"
          },
          {
            "name": "metrics/with_2_labels",
            "value": 186.26,
            "unit": "ns/op"
          },
          {
            "name": "metrics/with_4_labels",
            "value": 308.42,
            "unit": "ns/op"
          }
        ]
      }
    ],
    "condition": [
      {
        "commit": {
          "author": {
            "email": "fishlegs.engerman@gmail.com",
            "name": "f1shl3gs",
            "username": "f1shl3gs"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "49dfea83db53aeefe58e59116dfd484c0852e188",
          "message": "Merge pull request #710 from f1shl3gs/fix_bench_charts_name\n\nfix bench name",
          "timestamp": "2023-01-14T22:59:04+08:00",
          "tree_id": "7abdd153c16fac584f3033b1748d455a947b0c3c",
          "url": "https://github.com/f1shl3gs/vertex/commit/49dfea83db53aeefe58e59116dfd484c0852e188"
        },
        "date": 1673708611627,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "condition/ordering",
            "value": 22.617,
            "unit": "ns/op"
          },
          {
            "name": "condition/contains",
            "value": 22.196,
            "unit": "ns/op"
          },
          {
            "name": "condition/nested",
            "value": 23.166,
            "unit": "ns/op"
          },
          {
            "name": "condition/match",
            "value": 31.03,
            "unit": "ns/op"
          },
          {
            "name": "",
            "value": 50.1,
            "unit": "ns/op"
          }
        ]
      }
    ],
    "tracing-limit": [
      {
        "commit": {
          "author": {
            "email": "fishlegs.engerman@gmail.com",
            "name": "f1shl3gs",
            "username": "f1shl3gs"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "49dfea83db53aeefe58e59116dfd484c0852e188",
          "message": "Merge pull request #710 from f1shl3gs/fix_bench_charts_name\n\nfix bench name",
          "timestamp": "2023-01-14T22:59:04+08:00",
          "tree_id": "7abdd153c16fac584f3033b1748d455a947b0c3c",
          "url": "https://github.com/f1shl3gs/vertex/commit/49dfea83db53aeefe58e59116dfd484c0852e188"
        },
        "date": 1673708623054,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "tracing-limit/none/1",
            "value": 481.18,
            "unit": "ns/op"
          },
          {
            "name": "tracing-limit/none/100",
            "value": 48252,
            "unit": "ns/op"
          },
          {
            "name": "tracing-limit/none/500",
            "value": 240570,
            "unit": "ns/op"
          },
          {
            "name": "tracing-limit/none/1000",
            "value": 480810,
            "unit": "ns/op"
          },
          {
            "name": "tracing-limit/5s/1",
            "value": 268.64,
            "unit": "ns/op"
          },
          {
            "name": "tracing-limit/5s/100",
            "value": 26485,
            "unit": "ns/op"
          },
          {
            "name": "tracing-limit/5s/500",
            "value": 132170,
            "unit": "ns/op"
          },
          {
            "name": "tracing-limit/5s/1000",
            "value": 264440,
            "unit": "ns/op"
          }
        ]
      }
    ],
    "Build": [
      {
        "commit": {
          "author": {
            "email": "fishlegs.engerman@gmail.com",
            "name": "f1shl3gs",
            "username": "f1shl3gs"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "49dfea83db53aeefe58e59116dfd484c0852e188",
          "message": "Merge pull request #710 from f1shl3gs/fix_bench_charts_name\n\nfix bench name",
          "timestamp": "2023-01-14T22:59:04+08:00",
          "tree_id": "7abdd153c16fac584f3033b1748d455a947b0c3c",
          "url": "https://github.com/f1shl3gs/vertex/commit/49dfea83db53aeefe58e59116dfd484c0852e188"
        },
        "date": 1673709585299,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Baseline",
            "value": 1075,
            "unit": "s"
          },
          {
            "name": "Binary size",
            "value": 56930896,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}