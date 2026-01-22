# vsaas-xm-yolo-grpc-client

Simpler version of [xm-yolo-offsite](https://github.com/xompass/xompass-xm-yolo-offsite).

## XEDGE Parameters

None. Use program params.

Program params:

```
vsaas-xm-yolo-grpc-client 0.0.0

USAGE:
    vsaas-xm-yolo-grpc-client [OPTIONS] --backpressure <backpressure> --grpc-timeout-s <grpc-timeout-s>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --backpressure <backpressure>                      how many inputs can be processed concurrently
        --bridge-base <bridge-base>
            vsaas bridge api base url [default: https://bridge.xompass.com]

    -t, --grpc-timeout-s <grpc-timeout-s>                  timeout for grpc in secs
        --grpc-url <grpc-url>...                           one or more grpc urls
        --token-from-credential <token-from-credential>    credential id @ vsaas.ai
        --token-timeout-s <token-timeout-s>                timeout for retrieving token in secs [default: 10]

```

## XEDGE Routes

### Sink: detections

Same format as [xm-yolo-offsite](https://github.com/xompass/xompass-xm-yolo-offsite).

### Gate: image

Jsonmeta image. Same format as meta-image in [xm-yolo-offsite](https://github.com/xompass/xompass-xm-yolo-offsite).

## Testing

```
./test.sh image.jpg
```

---

## Build

As any other rust project:

```bash
cargo build
```

To make docker images for `aarch64` and `x86_64`

```bash
make
```

To make and push docker manifest with all archs:

```bash
make manifest
```

## Template

This project was initialized using this [template](https://github.com/xompass/xompass-edge-module-rs).

More on [cargo-generate](https://github.com/cargo-generate/cargo-generate).

Add me as a favorite:

```toml
# ~/.cargo/cargo-generate.toml
[favorites.xm]
description = "Xompass Edge Module"
git = "git@github.com:xompass/xompass-edge-module-rs.git"
# branch = "<optional-branch>"
# subfolder = "<optional-subfolder>"
```

Now you can:

```bash
cargo generate xm
```
