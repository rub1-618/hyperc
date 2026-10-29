FROM ubuntu:24.04

WORKDIR /work

RUN <<EOF
set -eux
apt-get update
apt-get install -y wget build-essential lsb-release software-properties-common gnupg
wget -O /work/llvm.sh https://apt.llvm.org/llvm.sh
chmod +x /work/llvm.sh
test -s /work/llvm.sh
/work/llvm.sh 22
wget -O /tmp/rustup-init.sh https://sh.rustup.rs
sh /tmp/rustup-init.sh -y --profile minimal
apt-get install -y llvm-22-dev libpolly-22-dev clang-22 libzstd-dev zlib1g-dev
EOF

ENV PATH="/root/.cargo/bin:${PATH}"

RUN <<EOF
rustup --version
cargo --version
rustc --version
clang-22 --version
llvm-as --version
llvm-config-22 --version
EOF

COPY . /work

RUN cargo test --release

RUN cargo build --release

ENTRYPOINT ["/usr/local/bin/hyperc"]
