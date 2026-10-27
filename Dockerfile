FROM ubuntu:24.04

WORKDIR /work

RUN <<EOF
apt-get update
apt-get install -y wget rustup build-essential
wget https://apt.llvm.org/llvm.sh
chmod +x ./llvm.sh
./llvm.sh 22
rustup install stable
apt-get install -y llvm-22-dev libpolly-22-dev clang-22 libzstd-dev
EOF

ENV PATH="/root/.cargo/bin:${PATH}"

RUN <<EOF
rustup --version
cargo --version
rustc --version
clang-22 --version
llvm-config-22 --version
EOF

COPY . /work

RUN cargo test --release

RUN cargo build --release

ENTRYPOINT ["/work/target/release/hyperc"]