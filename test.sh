#!/bin/bash

image="$1"
if [ -z "$image" ]; then
    >&2 echo "usage: $0 image"
    exit 1
fi

if [ -z "$XEDGE_DEVICE_TOKEN" ]; then
    XEDGE_DEVICE_TOKEN=$(for cont in $(docker ps -q); do
        envs=$(docker inspect "$cont" | jq .[0].Config.Env)
        if ! echo "$envs" | grep XEDGE_SYNC=stg >/dev/null; then
            echo "$envs" | grep XEDGE_DEVICE_TOKEN | cut -d= -f2 | sed 's/",*//g'
        fi
    done | head -n1)
    if [ -z "$XEDGE_DEVICE_TOKEN" ]; then
        >&2 echo "Missing XEDGE_DEVICE_TOKEN. Failed to extract from running containers."
        exit 1
    else
        >&2 echo "Extracted XEDGE_DEVICE_TOKEN from running agent."
    fi
fi
export XEDGE_DEVICE_TOKEN

export XEDGE_MODULE_NAME=yolo-grpc-client
ROUTING="xedge/default/modules/$XEDGE_MODULE_NAME/routing"
FEEDER_NAME=yolo-grpc-client-feeder
FEEDER_SINK="xedge/default/modules/$FEEDER_NAME/sinks/image"

cleanup() {
    set -x
    mosquitto_pub -t $ROUTING -r -n
    mosquitto_pub -t $FEEDER_SINK -r -n
}

trap cleanup EXIT

mosquitto_pub -t $ROUTING -r -m '{"image":{"yolo-grpc-client-feeder":["image"]}}'
echo -n '{"asset_id":"test","ts":"'$(date --iso-8601=s)'"}' \
    | cat - "$image" \
    | mosquitto_pub -t $FEEDER_SINK -r -s

timeout 10 mosquitto_sub -t "xedge/default/modules/$XEDGE_MODULE_NAME/sinks/detections" -C 1 | python3 draw_detections.py -o out.jpg && xdg-open out.jpg  &

cargo run -- --backpressure 1 -t 10 --grpc-url "https://grpc-gp.vsaas.ai" --token-from-credential grpc-gp
#cargo run -- --backpressure 1 -t 10 --grpc-url "https://grpc.xompass.com"
