import collections
import json
import sys


CREATE_PREFIXES = ("DeviceBase::APICreate", "ShaderModuleVk::GetHandleAndSpirv", "tint::")


def windows(marks):
    end = marks.get("flightDone") or marks.get("settled")
    cold_end = marks.get("warmDone") or end
    spans = [("cold", marks["appReady"], cold_end)]
    if marks.get("warmDone"):
        spans.append(("measured", marks["warmDone"], end))
    return spans


def main(trace_path, gaps_path):
    trace = json.load(open(trace_path))["traceEvents"]
    gaps = json.load(open(gaps_path))
    t0, marks = gaps["t0"], gaps["marks"]
    procs = {}
    for event in trace:
        if event.get("ph") == "M" and event.get("name") == "process_name":
            procs[event["pid"]] = event["args"]["name"]
    start = [event for event in trace if event.get("name") == "trace-start"][0]["ts"]
    gpu_spans = [event for event in trace if event.get("ph") == "X" and "dur" in event and procs.get(event["pid"]) == "GPU Process" and event.get("name", "").startswith(CREATE_PREFIXES)]
    for label, a_ms, b_ms in windows(marks):
        a = start + (a_ms - t0) * 1000.0
        b = start + (b_ms - t0) * 1000.0
        count = collections.Counter()
        total = collections.Counter()
        for event in gpu_spans:
            if event["ts"] >= a and event["ts"] + event["dur"] <= b:
                kind = event["name"].split("::")[0] + "::" + event["name"].split("::")[1][:24]
                count[kind] += 1
                total[kind] += event["dur"] / 1000.0
        sum_ms = sum(total.values())
        print(label, "window_ms", round(b_ms - a_ms), "creates", sum(count.values()), "gpu_create_ms", round(sum_ms, 1))
        for kind in sorted(total, key=lambda k: -total[k])[:6]:
            print("   ", kind, count[kind], round(total[kind], 1))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
