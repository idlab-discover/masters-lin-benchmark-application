#!/usr/bin/env python3

import time
import statistics
import requests
import matplotlib.pyplot as plt
from scipy.stats import f_oneway

def do_warmup(base_url, size, warmup_rounds=5):
    """
    Perform a few 'warm-up' rounds for a given size
    to let caching, JIT, etc. settle. We do not collect or return these times.
    """
    for _ in range(warmup_rounds):
        _ = requests.get(f"{base_url}/?size={size}")

def measure_time(base_url, size, trials=10):
    """
    Make `trials` GET requests to /?size=SIZE, measure each round-trip time in ms.
    Return a list of times in ms.
    """
    times_ms = []
    for _ in range(trials):
        start = time.time()
        resp = requests.get(f"{base_url}/?size={size}")
        end = time.time()

        elapsed_ms = (end - start) * 1000.0
        times_ms.append(elapsed_ms)

        if resp.status_code != 200:
            print(f"Warning: got status={resp.status_code}, text={resp.text}")
    return times_ms

def main():
    base_url = "http://localhost:8000"  
    sizes = [1_000_000, 2_000_000, 5_000_000, 10_000_000, 20_000_000]

    # Store results in a dictionary: size -> list_of_times_ms
    results = {}
    stats_summary = []  # To store means and stddevs for summary table

    # Warm up once for each size, then measure
    warmup_rounds = 5
    real_trials = 100

    for s in sizes:
        print(f"Warm-up for size={s}")
        do_warmup(base_url, s, warmup_rounds)

    for s in sizes:
        print(f"Measuring size={s} with {real_trials} trials...")
        times = measure_time(base_url, s, trials=real_trials)
        results[s] = times
        # Show basic stats
        avg = statistics.mean(times)
        stdev = statistics.pstdev(times)  # population stdev
        stats_summary.append((s, avg, stdev))
        print(f"  Times (ms): {times}")
        print(f"  Mean={avg:.3f}ms, StdDev={stdev:.3f}ms\n")

    # --- ANOVA test to see if means differ significantly ---
    groups = [results[s] for s in sizes]
    stat, pvalue = f_oneway(*groups)
    print(f"ANOVA test: F-stat={stat:.3f}, p-value={pvalue:.5f}")
    if pvalue < 0.05:
        print("=> There IS a statistically significant difference among the size groups (p < 0.05)")
    else:
        print("=> No statistically significant difference among the size groups (p >= 0.05)")

    print("\nSummary of Results (Mean and StdDev for Each Size):")
    print(f"{'Size (bytes)':<15}{'Mean (ms)':<15}{'StdDev (ms)':<15}")
    print("-" * 45)
    for size, mean, stddev in stats_summary:
        print(f"{size:<15}{mean:<15.3f}{stddev:<15.3f}")

    plt.figure(figsize=(8,6))
    for s in sizes:
        yvals = results[s]
        xvals = [s]*len(yvals)
        plt.scatter(xvals, yvals, label=f"{s} bytes" if s == sizes[0] else None)

    plt.title("Instantiation/Latency vs. Requested Size")
    plt.xlabel("Requested Size (bytes)")
    plt.ylabel("Round-trip time (ms)")
    plt.grid(True)
    plt.legend(["Samples"], loc="best")
    plt.savefig("latency_vs_size.png")

if __name__ == "__main__":
    main()

