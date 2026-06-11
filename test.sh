#!/bin/bash

set -eo pipefail

echoerr() {
    >&2 echo "$@"
}

init_ready=false
recv_ready=false
payload_sent=false

export XEDGE_MODULE_NAME=offsite
export RUST_LOG=debug
params="
--backpressure 10
--grpc-timeout-s 10
--grpc-url https://grpc.xompass.com
--xedge-authentication
"
cargo run -- $params 2>&1 | while read line; do
    echoerr "$line"
    case "$line" in
        *"Subscribing to xedge/default/modules/$XEDGE_MODULE_NAME/parameters"*)
            # this module receives no xedge parameters
            ;;
        *"Subscribing to xedge/default/modules/$XEDGE_MODULE_NAME/routing"*)
            mosquitto_pub -t xedge/default/modules/$XEDGE_MODULE_NAME/routing -m '{
                "image": {
                    "producer": ["o"]
                }
            }'
            ;;
        *"Subscribing to xedge/default/modules/producer/sinks/o"*)
            recv_ready=true
            init_ready=true
            ;;
        *"Publishing into xedge/default/modules/$XEDGE_MODULE_NAME/sinks/detections"*)
            echoerr "$0: stop execution with ^C"
            break
            ;;
    esac

    if ! $payload_sent && $recv_ready && $init_ready; then
        set -x
        echo '{
            "ts": 0,
            "asset_id": "test"
        }'  | jq -cj \
            | cat - dog.jpg \
            | mosquitto_pub -t xedge/default/modules/producer/sinks/o -s
        mosquitto_sub -t xedge/default/modules/$XEDGE_MODULE_NAME/sinks/detections -C 1 \
            | python3 strip_jsonmeta.py \
            | display - &
        payload_sent=true
        set +x
    fi
done
