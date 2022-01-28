ARG APP_NAME=aegisd

FROM rust:1.58 as builder
ARG APP_NAME

COPY . ./
RUN cargo fetch
RUN cargo build --release -p ${APP_NAME}

FROM debian:bullseye-slim
ARG APP_NAME
ARG CONFIG_FILE=/etc/${APP_NAME}/${APP_NAME}.conf

COPY --from=builder target/release/${APP_NAME} .

ENV CONFIG_FILE ${CONFIG_FILE}
ENV APP_NAME ${APP_NAME}

CMD ./${APP_NAME} -c ${CONFIG_FILE}
