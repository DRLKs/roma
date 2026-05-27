FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=/usr/local/cargo/bin:${PATH}
ENV ROMA_QAP_GA_IN_CONTAINER=1

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
COPY benchmark_suite/common.py ./benchmark_suite/common.py
COPY benchmark_suite/reporting.py ./benchmark_suite/reporting.py
COPY benchmark_suite/qap_ga ./benchmark_suite/qap_ga

RUN python3 -m pip install --break-system-packages --no-cache-dir -r /workspace/benchmark_suite/qap_ga/requirements.txt

RUN python3 -m venv /opt/mealpy-venv \
	&& /opt/mealpy-venv/bin/pip install --no-cache-dir --upgrade pip setuptools wheel \
	&& /opt/mealpy-venv/bin/pip install --no-cache-dir -r /workspace/benchmark_suite/qap_ga/requirements-mealpy.txt

RUN cargo build --release --example qap_ga_benchmark --manifest-path /workspace/roma/Cargo.toml \
	&& cp /workspace/roma/target/release/examples/qap_ga_benchmark /usr/local/bin/roma_qap_ga_benchmark

RUN mkdir -p /workspace/benchmark_suite/qap_ga/.cache/jmetal_java/lib \
	&& curl -fsSL -o /workspace/benchmark_suite/qap_ga/.cache/jmetal_java/lib/jmetal-core-5.11-jar-with-dependencies.jar https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-core/5.11/jmetal-core-5.11-jar-with-dependencies.jar \
	&& curl -fsSL -o /workspace/benchmark_suite/qap_ga/.cache/jmetal_java/lib/jmetal-algorithm-5.11-jar-with-dependencies.jar https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-algorithm/5.11/jmetal-algorithm-5.11-jar-with-dependencies.jar \
	&& curl -fsSL -o /workspace/benchmark_suite/qap_ga/.cache/jmetal_java/lib/jmetal-problem-5.11-jar-with-dependencies.jar https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-problem/5.11/jmetal-problem-5.11-jar-with-dependencies.jar

RUN mkdir -p /workspace/benchmark_suite/qap_ga/.cache/jmetal_java/classes \
	&& javac \
		-cp /workspace/benchmark_suite/qap_ga/.cache/jmetal_java/lib/jmetal-core-5.11-jar-with-dependencies.jar:/workspace/benchmark_suite/qap_ga/.cache/jmetal_java/lib/jmetal-algorithm-5.11-jar-with-dependencies.jar:/workspace/benchmark_suite/qap_ga/.cache/jmetal_java/lib/jmetal-problem-5.11-jar-with-dependencies.jar \
		-d /workspace/benchmark_suite/qap_ga/.cache/jmetal_java/classes \
		/workspace/benchmark_suite/qap_ga/runners/java/QapGeneticAlgorithmBenchmark.java

RUN mkdir -p /workspace/benchmark_suite/qap_ga/.cache/pagmo_cpp \
	&& g++ -O2 -std=c++17 /workspace/benchmark_suite/qap_ga/runners/cpp/qap_pagmo_benchmark.cpp -o /workspace/benchmark_suite/qap_ga/.cache/pagmo_cpp/qap_pagmo_benchmark -lpagmo

ENTRYPOINT ["python3", "/workspace/benchmark_suite/qap_ga/orchestrate.py"]