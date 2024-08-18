FROM rustlang/rust:nightly-alpine AS builder

RUN apk add --no-cache \
                       cmake \
                       gcc \
                       git \
                       libarchive-tools \
                       linux-headers \
                       make \
                       musl-dev \
                       unixodbc \
                       unixodbc-static \
                       unzip

WORKDIR /src/odbc-ini-gen
ADD https://github.com/SamuelMarks/odbc-ini-gen/archive/refs/heads/master.zip .
RUN bsdtar xvf master.zip --strip-components=1 && \
    cmake -S . -B build_alpine && \
    cmake --build build_alpine && \
    cmake --install build_alpine --prefix /opt/odbc-ini-gen && \
    printf 'Installed odbc-ini-gen %s' "$(/opt/odbc-ini-gen/bin/odbc_ini_gen --version)"

WORKDIR /src/odbc-cli
COPY . .
RUN cargo build --release

FROM alpine AS runner
ENV DEFAULT_DB_DRIVER="PostgreSQL UNICODE"

RUN apk add --no-cache curl jq unixodbc unixodbc-static psqlodbc wait4x && \
    apk add --no-cache sqliteodbc --repository=https://dl-cdn.alpinelinux.org/alpine/edge/testing

COPY --from=builder /src/odbc-cli/target/release/odbc-cli /usr/local/bin/
COPY --from=builder /opt/odbc-ini-gen/bin/odbc_ini_gen /usr/local/bin/

RUN /usr/local/bin/odbc_ini_gen --infer-all -o /etc/odbcinst.ini && \
    printf '[Default]\nDriver = %s\n' "${DEFAULT_DB_DRIVER}" > /etc/odbc.ini

ENTRYPOINT ["/usr/local/bin/odbc-cli"]
