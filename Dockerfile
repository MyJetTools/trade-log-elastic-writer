FROM ubuntu:22.04
COPY ./target/release/trade-log-elastic-writer ./target/release/trade-log-elastic-writer

RUN apt-get update && apt-get install -y ca-certificates

ARG app_version
ARG app_compilation_date
ENV APP_VERSION=${app_version}
ENV APP_COMPILATION_DATE=${app_compilation_date}

ENTRYPOINT ["./target/release/trade-log-elastic-writer"]