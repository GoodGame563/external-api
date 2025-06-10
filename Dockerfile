# Stage 1: Build
FROM rust:1.86-slim-bullseye as builder

WORKDIR /app

# Установка зависимостей сборки
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev protobuf-compiler && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Кэшируем зависимости
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -r src

# Копируем исходники
COPY . .

# Собираем проект
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bullseye-slim

WORKDIR /app

# Устанавливаем только нужные runtime-зависимости
RUN apt-get update && \
    apt-get install -y libpq5 libssl1.1 && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Копируем бинарник и нужные файлы
COPY --from=builder /app/target/release/external-api /app/external-api
COPY --from=builder /app/proto /app/proto
COPY --from=builder /app/docker.env /app/.env

EXPOSE 8000

CMD ["./external-api"]