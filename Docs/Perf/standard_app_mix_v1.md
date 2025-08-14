mix_id: "std-app-mix-v1"
desktop:
  processes:
    - name: "RaeUI + compositor"
      workload: "idle + window moves every 5s"
    - name: "Browser"
      tabs: 10
      media: "1x 1080p60"
    - name: "Filesync (idle)"
    - name: "Editor"
      project: "rae-sample"
laptop:
  extra:
    - name: "Wi-Fi roam every 2–5 min"
      constraints: "no audio underruns"
data_sets:
  browser_cache_mb: 512
  file_io_hotset_mb: 256
timings:
  warmup_s: 45
  duration_s: 600

