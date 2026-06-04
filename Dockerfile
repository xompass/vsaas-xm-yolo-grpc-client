FROM alpine as ca

RUN apk add -U --no-cache \
    ca-certificates

FROM scratch

COPY --from=ca /etc/ssl/certs /etc/ssl/certs

ARG BIN
COPY $BIN /$BIN

ENV RUST_LOG=info

# BIN is not replaced because needs shell in runtime
# which scratch does not have. Hack implemented in Makefile.
ENTRYPOINT [ "${BIN}" ]
