FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=/usr/local/cargo/bin:${PATH}
ENV ROMA_KNAPSACK_SA_IN_CONTAINER=1

WORKDIR /workspace

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		build-essential \
		ca-certificates \
		curl \
		git \
		python3-venv \
		python3-pip \
		python3 \
	&& rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable

COPY roma ./roma
COPY benchmark_suite/common.py ./benchmark_suite/common.py
COPY benchmark_suite/reporting.py ./benchmark_suite/reporting.py
COPY benchmark_suite/knapsack_sa ./benchmark_suite/knapsack_sa

RUN python3 -m pip install --break-system-packages --no-cache-dir -r /workspace/benchmark_suite/knapsack_sa/requirements.txt

RUN python3 -m venv /opt/mealpy-venv \
	&& /opt/mealpy-venv/bin/pip install --no-cache-dir --upgrade pip setuptools wheel \
	&& /opt/mealpy-venv/bin/pip install --no-cache-dir -r /workspace/benchmark_suite/knapsack_sa/requirements-mealpy.txt || true

RUN cargo build --release --example knapsack_sa_benchmark --manifest-path /workspace/roma/Cargo.toml \
	&& cp /workspace/roma/target/release/examples/knapsack_sa_benchmark /usr/local/bin/roma_knapsack_sa_benchmark || true

ENTRYPOINT ["python3", "/workspace/benchmark_suite/knapsack_sa/orchestrate.py"]
