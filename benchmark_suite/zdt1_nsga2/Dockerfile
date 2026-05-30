FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=/usr/local/cargo/bin:${PATH}
ENV ROMA_ZDT1_NSGA2_IN_CONTAINER=1

WORKDIR /workspace

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		build-essential \
		ca-certificates \
		curl \
		git \
		libboost-serialization-dev \
		libpagmo-dev \
		openjdk-21-jdk-headless \
		pkg-config \
		python3-venv \
		python3-pip \
		python3 \
	&& rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable

COPY roma ./roma
COPY benchmark_suite/*.py ./benchmark_suite/
COPY benchmark_suite/zdt1_nsga2 ./benchmark_suite/zdt1_nsga2

RUN python3 -m pip install --break-system-packages --no-cache-dir -r /workspace/benchmark_suite/zdt1_nsga2/requirements.txt

RUN cargo build --release --example zdt1_nsga2_benchmark --manifest-path /workspace/roma/Cargo.toml \
	&& cp /workspace/roma/target/release/examples/zdt1_nsga2_benchmark /usr/local/bin/roma_zdt1_nsga2_benchmark

RUN mkdir -p /workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/lib \
	&& curl -fsSL -o /workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/lib/jmetal-core-5.11-jar-with-dependencies.jar https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-core/5.11/jmetal-core-5.11-jar-with-dependencies.jar \
	&& curl -fsSL -o /workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/lib/jmetal-algorithm-5.11-jar-with-dependencies.jar https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-algorithm/5.11/jmetal-algorithm-5.11-jar-with-dependencies.jar \
	&& curl -fsSL -o /workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/lib/jmetal-problem-5.11-jar-with-dependencies.jar https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-problem/5.11/jmetal-problem-5.11-jar-with-dependencies.jar

RUN mkdir -p /workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/classes \
	&& javac \
		-cp /workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/lib/jmetal-core-5.11-jar-with-dependencies.jar:/workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/lib/jmetal-algorithm-5.11-jar-with-dependencies.jar:/workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/lib/jmetal-problem-5.11-jar-with-dependencies.jar \
		-d /workspace/benchmark_suite/zdt1_nsga2/.cache/jmetal_java/classes \
		/workspace/benchmark_suite/zdt1_nsga2/runners/java/Zdt1NsgaIIBenchmark.java

RUN mkdir -p /workspace/benchmark_suite/zdt1_nsga2/.cache/pagmo_cpp \
	&& g++ -O2 -std=c++17 /workspace/benchmark_suite/zdt1_nsga2/runners/cpp/zdt1_pagmo_benchmark.cpp -o /workspace/benchmark_suite/zdt1_nsga2/.cache/pagmo_cpp/zdt1_pagmo_benchmark -lpagmo

ENTRYPOINT ["python3", "/workspace/benchmark_suite/zdt1_nsga2/orchestrate.py"]