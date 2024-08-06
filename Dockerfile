FROM rustlang/rust:nightly-alpine AS builder
#FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev unixodbc-static

WORKDIR /src/odbc-cli
COPY . .
RUN cargo build --release

FROM scratch AS runner
##FROM busybox:musl AS runner
#FROM alpine AS runner
#
#RUN apk add --no-cache unixodbc
COPY --from=builder /src/odbc-cli/target/release/odbc-cli /usr/local/bin/
#
ENTRYPOINT ["/usr/local/bin/odbc-cli"]
