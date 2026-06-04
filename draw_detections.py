import sys, argparse, json
from PIL import Image, ImageDraw
import io

parser = argparse.ArgumentParser(description="input jsonmeta, render bboxes, "\
        "write to -o")
parser.add_argument("-o", "--output", help="output path")
args = parser.parse_args()

def split_metaimg(payload):
    """Split custom-format payload into it's parsed json and image."""
    bracket_cnt = 0
    start = None
    stop = 0

    for i, b in enumerate(payload):
        if chr(b) == "{":
            bracket_cnt += 1

            if start is None:
                start = i

        if chr(b) == "}":
            bracket_cnt -= 1

        if start is not None and bracket_cnt == 0:
            stop = i + 1
            break

    return payload[start:stop], payload[stop:]

input_data = sys.stdin.buffer.read()
jsonb, imgb = split_metaimg(input_data)

detections = json.loads(jsonb)["data"]
print(json.dumps(detections, indent=2))

# Load image
img = Image.open(io.BytesIO(imgb))
img_draw = ImageDraw.Draw(img)
img_shape = img.size
# Draw image
for detection in detections:
    bbox = detection["frame"]
    img_draw.rectangle([bbox["x"] - bbox.get("w", 512)/2, bbox["y"] -
                        bbox.get("h", 512)/2,bbox["x"] + bbox.get("w", 512)/2,
                        bbox["y"] + bbox.get("h", 512)/2], width=2)

if args.output == "-":
    img.save(sys.stdout, format="jpeg")
else:
    img.save(args.output)
