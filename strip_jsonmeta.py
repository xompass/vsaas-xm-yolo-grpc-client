import sys
import json

def jsonmeta(payload):
    assert isinstance(payload, bytes)
    count = 0
    started = False
    for i, c in enumerate(payload):
        ch = chr(c)
        if not started and ch == '{':
            started = True
        if started:
            if ch == '{':
                count += 1
            elif ch == '}':
                count -= 1
                if count == 0:
                    return payload[:i+1], payload[i+1:]

metadata, data = jsonmeta(sys.stdin.buffer.read())
json.dump(json.loads(metadata), sys.stderr, indent=2)
sys.stdout.buffer.write(data)
