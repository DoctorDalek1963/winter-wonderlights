window.BENCHMARK_DATA = {
  "lastUpdate": 1685034019895,
  "repoUrl": "https://github.com/DoctorDalek1963/winter-wonderlights",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "dyson.dyson@icloud.com",
            "name": "DoctorDalek1963",
            "username": "DoctorDalek1963"
          },
          "committer": {
            "email": "dyson.dyson@icloud.com",
            "name": "DoctorDalek1963",
            "username": "DoctorDalek1963"
          },
          "distinct": true,
          "id": "96c62ece967b5cb2897c9b72f20791628ecc94d8",
          "message": "Commit `.env` for CI",
          "timestamp": "2023-05-24T22:22:51+01:00",
          "tree_id": "1b6657b3347d89b74517e05b10618370299ccc6f",
          "url": "https://github.com/DoctorDalek1963/winter-wonderlights/commit/96c62ece967b5cb2897c9b72f20791628ecc94d8"
        },
        "date": 1684963603787,
        "tool": "cargo",
        "benches": [
          {
            "name": "(SimpleDriver) DebugOneByOne",
            "value": 43054,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "(SimpleDriver) DebugBinaryIndex",
            "value": 150049,
            "range": "± 273",
            "unit": "ns/iter"
          },
          {
            "name": "(SimpleDriver) MovingPlane",
            "value": 31863,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "(SimpleDriver) LavaLamp",
            "value": 26621,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) DebugOneByOne",
            "value": 85664,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) DebugBinaryIndex",
            "value": 152780,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) MovingPlane",
            "value": 12863047,
            "range": "± 5973",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) LavaLamp",
            "value": 2935975,
            "range": "± 9431",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "dyson.dyson@icloud.com",
            "name": "DoctorDalek1963",
            "username": "DoctorDalek1963"
          },
          "committer": {
            "email": "dyson.dyson@icloud.com",
            "name": "DoctorDalek1963",
            "username": "DoctorDalek1963"
          },
          "distinct": true,
          "id": "798c9c7f126ad17b9d4d62b3e5f22b6c97dbe16b",
          "message": "Improve benchmarks to automatically update with new effects, and lower benchmark alert threshold in CI",
          "timestamp": "2023-05-25T11:55:17+01:00",
          "tree_id": "613612a39b221ef0c21a0afd1d877bfb15779fb1",
          "url": "https://github.com/DoctorDalek1963/winter-wonderlights/commit/798c9c7f126ad17b9d4d62b3e5f22b6c97dbe16b"
        },
        "date": 1685034018803,
        "tool": "cargo",
        "benches": [
          {
            "name": "(SimpleDriver) DebugOneByOne",
            "value": 52813,
            "range": "± 543",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) DebugOneByOne",
            "value": 93168,
            "range": "± 265",
            "unit": "ns/iter"
          },
          {
            "name": "(SimpleDriver) DebugBinaryIndex",
            "value": 178394,
            "range": "± 1415",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) DebugBinaryIndex",
            "value": 177104,
            "range": "± 1070",
            "unit": "ns/iter"
          },
          {
            "name": "(SimpleDriver) MovingPlane",
            "value": 37907,
            "range": "± 634",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) MovingPlane",
            "value": 15180984,
            "range": "± 182311",
            "unit": "ns/iter"
          },
          {
            "name": "(SimpleDriver) LavaLamp",
            "value": 31912,
            "range": "± 373",
            "unit": "ns/iter"
          },
          {
            "name": "(ConvertFrameDriver) LavaLamp",
            "value": 3543398,
            "range": "± 2005",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}