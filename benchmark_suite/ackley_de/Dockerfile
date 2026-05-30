FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=/usr/local/cargo/bin:${PATH}
ENV ROMA_ACKLEY_DE_IN_CONTAINER=1

WORKDIR /workspace

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		build-essential \
		ca-certificates \
		curl \
		git \
		libboost-serialization-dev \
		libpagmo-dev \
		pkg-config \
		python3-venv \
		python3-pip \
		python3 \
	&& rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable

COPY roma ./roma
COPY benchmark_suite/*.py ./benchmark_suite/
COPY benchmark_suite/ackley_de ./benchmark_suite/ackley_de

RUN python3 -m pip install --break-system-packages --no-cache-dir -r /workspace/benchmark_suite/ackley_de/requirements.txt

RUN cargo build --release --example ackley_de_benchmark --manifest-path /workspace/roma/Cargo.toml \
	&& cp /workspace/roma/target/release/examples/ackley_de_benchmark /usr/local/bin/roma_ackley_de_benchmark

RUN mkdir -p /workspace/benchmark_suite/ackley_de/.cache/pagmo_cpp \
	&& g++ -O2 -std=c++17 /workspace/benchmark_suite/ackley_de/runners/cpp/ackley_pagmo_benchmark.cpp -o /workspace/benchmark_suite/ackley_de/.cache/pagmo_cpp/ackley_pagmo_benchmark -lpagmo

ENTRYPOINT ["python3", "/workspace/benchmark_suite/ackley_de/orchestrate.py"]
