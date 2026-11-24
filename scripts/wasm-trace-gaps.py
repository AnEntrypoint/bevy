import collections
import json
import sys


def main(trace_path, gaps_path, limit):
    events = json.load(open(trace_path))["traceEvents"]
    gaps = json.load(open(gaps_path))
    procs, threads = {}, {}
    for event in events:
        if event.get("ph") == "M" and event.get("name") == "process_name":
            procs[event["pid"]] = event["args"]["name"]
        if event.get("ph") == "M" and event.get("name") == "thread_name":
            threads[(event["pid"], event["tid"])] = event["args"]["name"]
    start = [event for event in events if event.get("name") == "trace-start"][0]["ts"]
    spans = [event for event in events if event.get("ph") == "X" and "dur" in event]
    gpu_pids = {pid for pid, name in procs.items() if name == "GPU Process"}
    for start_ms, dt, gap, busy in gaps["gaps"]:
        if start_ms < gaps["t0"]:
            continue
        a = start + (start_ms - gaps["t0"]) * 1000.0
        b = a + dt * 1000.0
        print("WINDOW start_ms", start_ms, "dt", dt, "gap", gap, "busy", busy)
        per_thread = collections.Counter()
        for event in spans:
            s, f = event["ts"], event["ts"] + event["dur"]
            if f < a or s > b:
                continue
            overlap = (min(f, b) - max(s, a)) / 1000.0
            if overlap <= 0:
                continue
            per_thread[(procs.get(event["pid"], str(event["pid"])), threads.get((event["pid"], event["tid"]), str(event["tid"])))] += overlap
        for key, value in per_thread.most_common(limit):
            print("  %7.1f ms  %-14s %s" % (value, key[0], key[1]))
        gpu_rows = []
        for event in spans:
            if event["pid"] not in gpu_pids:
                continue
            s, f = event["ts"], event["ts"] + event["dur"]
            if f < a or s > b:
                continue
            overlap = (min(f, b) - max(s, a)) / 1000.0
            if overlap >= 1.0:
                gpu_rows.append((round(overlap, 1), event.get("name")))
        gpu_rows.sort(reverse=True)
        for row in gpu_rows[:limit]:
            print("  gpu", row)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 10)
