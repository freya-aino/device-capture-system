#!/usr/bin/env bash
# Measurement run for docs/steps/step-26-multi-camera-run.md.
#
# Runs against an already-running hub (and its producers): reads the raw stream and, when a sync
# port is given, the aligned stream with `access-client` in summary mode for RUN_SECS, polls the
# hub's GET /status (producer/consumer counts, frames/s, Mbit/s, hub CPU) every STATUS_SECS, and
# writes one Markdown results note with the per-camera, per-group and hub tables plus the raw logs.
#
# Everything is configured by environment variables so the same script runs on the consumer host
# of a one-, two- or three-host deployment:
#
#   HUB_IP          hub address the clients connect to            (default 127.0.0.1)
#   HUB_PORT        raw XPUB port                                 (default 5556)
#   SYNC_PORT       HUB_SYNC_PORT of the hub, 0 = skip aligned     (default 5557)
#   HUB_STATUS_URL  the hub's status URL, empty = skip hub table   (default http://127.0.0.1:8081/status)
#   RUN_SECS        measurement length                            (default 60)
#   SUMMARY_SECS    access-client summary window                  (default 10)
#   STATUS_SECS     status poll interval                          (default 5)
#   LABEL           one line describing the setup (hosts, cameras, modes, chunk sizes)
#   OUT             results note path (default docs/hardware/<date>-multi-camera-run.md)
#   ACCESS_CLIENT   access-client binary (default: build target/release/access-client)
#   CLIENT_DATA_CHUNK_SIZE, CLIENT_QUEUE_SIZE  passed through to access-client
#
# Example, consumer host of a two-producer-host setup:
#   HUB_IP=10.0.0.2 HUB_STATUS_URL=http://10.0.0.2:8081/status \
#   LABEL="2 cameras, 1080p MJPG, 1 KiB parts" scripts/multi-camera-run.sh
set -euo pipefail

HUB_IP=${HUB_IP:-127.0.0.1}
HUB_PORT=${HUB_PORT:-5556}
SYNC_PORT=${SYNC_PORT:-5557}
HUB_STATUS_URL=${HUB_STATUS_URL-http://127.0.0.1:8081/status}
RUN_SECS=${RUN_SECS:-60}
SUMMARY_SECS=${SUMMARY_SECS:-10}
STATUS_SECS=${STATUS_SECS:-5}
LABEL=${LABEL:-}
DATE=$(date +%F)
OUT=${OUT:-docs/hardware/${DATE}-multi-camera-run.md}

repo=$(cd "$(dirname "$0")/.." && pwd)
if [ -z "${ACCESS_CLIENT:-}" ]; then
    (cd "$repo" && cargo build --release -p access-client >/dev/null)
    ACCESS_CLIENT=$repo/target/release/access-client
fi
[ -x "$ACCESS_CLIENT" ] || { echo "access-client binary not found: $ACCESS_CLIENT" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fetch_status() {
    curl -sf --max-time 2 "$HUB_STATUS_URL" 2>/dev/null || echo ""
}

if [ -n "$HUB_STATUS_URL" ]; then
    first_status=$(fetch_status)
    if [ -z "$first_status" ]; then
        echo "warning: no answer from $HUB_STATUS_URL, the hub table will be empty" >&2
    else
        echo "hub before the run: $first_status"
    fi
fi

echo "measuring for ${RUN_SECS}s: raw tcp://$HUB_IP:$HUB_PORT$([ "$SYNC_PORT" != 0 ] && echo ", aligned port $SYNC_PORT")"

HUB_IP=$HUB_IP HUB_PORT=$HUB_PORT CLIENT_SUMMARY_SECS=$SUMMARY_SECS CLIENT_RUN_SECS=$RUN_SECS \
    CLIENT_ALIGNED_PORT=0 "$ACCESS_CLIENT" >"$work/raw.log" 2>&1 &
raw_pid=$!

aligned_pid=
if [ "$SYNC_PORT" != 0 ]; then
    HUB_IP=$HUB_IP CLIENT_ALIGNED_PORT=$SYNC_PORT CLIENT_SUMMARY_SECS=$SUMMARY_SECS CLIENT_RUN_SECS=$RUN_SECS \
        "$ACCESS_CLIENT" >"$work/aligned.log" 2>&1 &
    aligned_pid=$!
fi

: >"$work/status.log"
if [ -n "$HUB_STATUS_URL" ]; then
    end=$((SECONDS + RUN_SECS))
    while [ $SECONDS -lt $end ]; do
        status=$(fetch_status)
        [ -n "$status" ] && printf '%s %s\n' "$(date +%T)" "$status" >>"$work/status.log"
        sleep "$STATUS_SECS"
    done
fi

wait "$raw_pid" || echo "warning: raw access-client exited with an error (see the log in the note)" >&2
[ -n "$aligned_pid" ] && { wait "$aligned_pid" || echo "warning: aligned access-client exited with an error" >&2; }

# --- Tables -----------------------------------------------------------------------------------

# One row per device: the final line gives the mean rate and the total drops, the last summary line
# the last window's bandwidth, latency and format.
per_camera_table() {
    awk '
        /^summary: / {
            match($0, /device=[^ ]+/); id = substr($0, RSTART + 7, RLENGTH - 7)
            match($0, /mbit\/s=[^ ]+/); mbit[id] = substr($0, RSTART + 7, RLENGTH - 7)
            if (match($0, /latency_ms mean=[^ ]+ max=[^ ]+/)) {
                split(substr($0, RSTART, RLENGTH), l, /[= ]/); lat[id] = l[3] " / " l[5]
            } else lat[id] = "n/a"
            match($0, /shape=\[[^]]*\]/); shape[id] = substr($0, RSTART + 6, RLENGTH - 6)
            match($0, /format=[^ ]+/); fmt[id] = substr($0, RSTART + 7, RLENGTH - 7)
        }
        /^final: / {
            match($0, /device=[^ ]+/); id = substr($0, RSTART + 7, RLENGTH - 7)
            match($0, /total_frames=[^ ]+/); frames[id] = substr($0, RSTART + 13, RLENGTH - 13)
            match($0, /mean_rate=[^ ]+/); rate[id] = substr($0, RSTART + 10, RLENGTH - 10)
            match($0, /dropped_total=[^ ]+/); dropped = substr($0, RSTART + 14, RLENGTH - 14)
            order[++n] = id
        }
        END {
            if (n == 0) { print "_no frames received_"; exit }
            print "| Device | Mode | Frames | Rate | Mbit/s (last window) | Latency mean / max ms (last window) |"
            print "|---|---|---|---|---|---|"
            for (i = 1; i <= n; i++) {
                id = order[i]
                printf "| `%s` | %s %s | %s | %s | %s | %s |\n", id, shape[id], fmt[id], frames[id], rate[id], mbit[id], lat[id]
            }
            printf "\nDropped frames (receiver total, all devices): %s\n", dropped
        }' "$1"
}

aligned_table() {
    awk '
        BEGIN { skew_max = "0.0" }
        /^aligned: / {
            match($0, /skew_ms mean=[^ ]+ max=[^ ]+/); split(substr($0, RSTART, RLENGTH), s, /[= ]/)
            skew_mean = s[3]; if (s[5] + 0 > skew_max + 0) skew_max = s[5]
            n++; sum += s[3]
            if (match($0, /latency_ms mean=[^ ]+ max=[^ ]+/)) {
                split(substr($0, RSTART, RLENGTH), l, /[= ]/); lat = l[3] " / " l[5]
            }
            match($0, /rate=[^ ]+/); rate = substr($0, RSTART + 5, RLENGTH - 5)
            match($0, /frames=\[[^]]*\]/); frames = substr($0, RSTART + 8, RLENGTH - 9)
        }
        /^final: aligned/ || /total_groups=/ { last = $0 }
        END {
            if (n == 0) { print "_no aligned groups received_"; exit }
            print "| Groups/s (last window) | Skew mean ms (windows avg) | Skew max ms | Group latency mean / max ms (last window) | Frames per device (last window) |"
            print "|---|---|---|---|---|"
            printf "| %s | %.1f | %s | %s | %s |\n", rate, sum / n, skew_max, lat, frames
        }' "$1"
}

hub_table() {
    awk '
        function field(name,   r) {
            if (match($0, "\"" name "\":[^,}]+")) { r = substr($0, RSTART, RLENGTH); sub(/.*:/, "", r); return r }
            return ""
        }
        {
            p = field("producers"); c = field("consumers")
            cpu = field("cpu_percent"); mbit = field("mbit_per_s"); fps = field("frames_per_s")
            if (cpu != "null" && cpu != "") { n_cpu++; cpu_sum += cpu; if (cpu + 0 > cpu_max + 0) cpu_max = cpu }
            if (mbit != "") { n++; mbit_sum += mbit; if (mbit + 0 > mbit_max + 0) mbit_max = mbit; fps_sum += fps }
        }
        END {
            if (n == 0) { print "_no status samples_"; exit }
            print "| Producers | Consumers | Hub frames/s (mean) | Hub Mbit/s mean / max | Hub CPU % of one core, mean / max |"
            print "|---|---|---|---|---|"
            cpu_text = (n_cpu > 0) ? sprintf("%.0f / %.0f", cpu_sum / n_cpu, cpu_max) : "n/a"
            printf "| %s | %s | %.1f | %.1f / %.1f | %s |\n", p, c, fps_sum / n, mbit_sum / n, mbit_max, cpu_text
        }' "$1"
}

mkdir -p "$(dirname "$OUT")"
{
    echo "# Multi-camera run — $DATE"
    echo
    echo "Results for [\`docs/steps/step-26-multi-camera-run.md\`](../steps/step-26-multi-camera-run.md), collected by"
    echo "\`scripts/multi-camera-run.sh\` at \`$(cd "$repo" && git rev-parse --short HEAD 2>/dev/null || echo unknown)\`."
    echo
    echo "- Setup: ${LABEL:-_fill in: hosts, cameras, modes, part sizes_}"
    echo "- Hub: \`tcp://$HUB_IP:$HUB_PORT\` raw$([ "$SYNC_PORT" != 0 ] && echo ", aligned on port $SYNC_PORT"); status $([ -n "$HUB_STATUS_URL" ] && echo "\`$HUB_STATUS_URL\`" || echo "not polled")"
    echo "- Run: ${RUN_SECS} s, ${SUMMARY_SECS} s summary windows, status every ${STATUS_SECS} s"
    echo "- Client: queue ${CLIENT_QUEUE_SIZE:-128}, ${CLIENT_DATA_CHUNK_SIZE:-1024}-byte parts expected"
    echo
    echo "## Per camera (raw stream)"
    echo
    per_camera_table "$work/raw.log"
    if [ "$SYNC_PORT" != 0 ]; then
        echo
        echo "## Aligned groups"
        echo
        aligned_table "$work/aligned.log"
    fi
    if [ -n "$HUB_STATUS_URL" ]; then
        echo
        echo "## Hub"
        echo
        hub_table "$work/status.log"
    fi
    echo
    echo "## Conclusions"
    echo
    echo "_Fill in: is hub-side pre-processing usable across hosts (step 23)? Which \`HUB_SYNC_MAX_WAIT_MS\` default do the_"
    echo "_skew and latency numbers support (step 10)? Effect of the part size (\`DEVICE_NODE_DATA_CHUNK_SIZE\`, \`HUB_DATA_CHUNK_SIZE\`)?_"
    echo
    echo "## Raw output"
    echo
    echo '```'
    echo "# raw stream"; cat "$work/raw.log"
    if [ "$SYNC_PORT" != 0 ]; then echo; echo "# aligned stream"; cat "$work/aligned.log"; fi
    if [ -n "$HUB_STATUS_URL" ]; then echo; echo "# hub status"; cat "$work/status.log"; fi
    echo '```'
} >"$OUT"

echo "wrote $OUT"
